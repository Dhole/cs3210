use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::fmt;

use aarch64::*;

use pi::interrupt::{Controller, Interrupt};
use pi::timer::tick_in;

use crate::console::{kprint, kprintln};
use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::shell;
use crate::traps::TrapFrame;
use crate::IRQ;
use crate::SCHEDULER;
use crate::VMM;

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }

    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }
            // aarch64::wfe();
            aarch64::wfi();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        // Setup timer interrupt
        IRQ.register(
            Interrupt::Timer1,
            Box::new(|tf| {
                let id = SCHEDULER.switch(State::Ready, tf);
                kprintln!("TICK, Switch to {}", id);
                tick_in(TICK);
            }),
        );
        let mut controller = Controller::new();
        controller.enable(Interrupt::Timer1);
        tick_in(TICK);

        let mut tf = Box::new(TrapFrame::default());
        self.critical(|scheduler| scheduler.switch_to(&mut tf));

        // context_restore
        unsafe {
            asm!("mov x0, $0
                  mov sp, x0"
                 :: "r"(tf)
                 :: "volatile");
            asm!("bl context_restore" :::: "volatile");
            asm!("adr x0, _start
                  mov sp, x0"
                 :::: "volatile");
            asm!("mov x0, #0" :::: "volatile");
        }
        eret();
        loop {}
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        let mut process1 = Process::new().expect("new process");
        let mut tf = &mut process1.context;
        tf.ELR = start_shell1 as *const u64 as u64;
        tf.SPSR = (SPSR_EL1::M & 0b0000) | SPSR_EL1::F | SPSR_EL1::A | SPSR_EL1::D;
        tf.SP = process1.stack.top().as_u64();

        // let mut process2 = Process::new().expect("new process");
        // let mut tf = &mut process2.context;
        // tf.ELR = start_shell2 as *const u64 as u64;
        // tf.SPSR = (SPSR_EL1::M & 0b0000) | SPSR_EL1::F | SPSR_EL1::A | SPSR_EL1::D;
        // tf.SP = process2.stack.top().as_u64();

        let mut scheduler = Scheduler::new();
        scheduler.add(process1);
        // scheduler.add(process2);
        *self.0.lock() = Some(scheduler);
    }

    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    // pub fn test_phase_3(&self, proc: &mut Process){
    //     use crate::vm::{VirtualAddr, PagePerm};
    //
    //     let mut page = proc.vmap.alloc(
    //         VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);
    //
    //     let text = unsafe {
    //         core::slice::from_raw_parts(test_user_process as *const u8, 24)
    //     };
    //
    //     page[0..24].copy_from_slice(text);
    // }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        Self {
            processes: VecDeque::new(),
            last_id: None,
        }
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        let id = match self.last_id {
            None => 0,
            Some(core::u64::MAX) => {
                return None;
            }
            Some(last_id) => last_id + 1,
        };
        self.last_id = Some(id);
        process.context.TPIDR = id;
        self.processes.push_back(process);
        Some(id)
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        let mut process = match self.processes.pop_front() {
            None => return false,
            Some(process) => process,
        };
        match process.state {
            State::Running => {
                process.state = new_state;
                process.context = Box::new(*tf);
                self.processes.push_back(process);
                true
            }
            _ => {
                self.processes.push_front(process);
                false
            }
        }
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        let mut i = 0;
        while let Some(mut process) = self.processes.swap_remove_front(i) {
            if process.is_ready() {
                process.state = State::Running;
                *tf = *process.context;
                let id = process.context.TPIDR;
                self.processes.push_front(process);
                return Some(id);
            }
            self.processes.push_front(process);
            i += 1;
        }
        if let Some(process) = self.processes.pop_front() {
            self.processes.push_back(process);
        }
        return None;
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        if let Some(mut process) = self.processes.pop_front() {
            if let State::Running = process.state {
                process.state = State::Dead;
                return Some(process.context.TPIDR);
            } else {
                self.processes.push_front(process);
            }
        }
        None
    }
}

pub extern "C" fn test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}

pub extern "C" fn start_shell1() {
    loop {
        shell::shell("user1> ", &crate::FILESYSTEM);
    }
}

pub extern "C" fn start_shell2() {
    loop {
        shell::shell("user2> ", &crate::FILESYSTEM);
    }
}
