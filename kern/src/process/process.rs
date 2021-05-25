use alloc::boxed::Box;
use core::mem;
use shim::io;
use shim::path::Path;

use aarch64;

use crate::allocator::util::{align_down, align_up};
use crate::console::{kprint, kprintln};
use crate::param::*;
use crate::process::{Stack, State};
use crate::traps::TrapFrame;
use crate::vm::*;
use aarch64::*;
use core::ptr::Unique;
use fat32::traits::Entry;
use fat32::traits::File;
use fat32::traits::FileSystem;
use kernel_api::{OsError, OsResult};
use shim::io::{Read, Seek};

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
#[derive(Debug)]
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    // pub stack: Stack,
    // pub stack: Unique<[u8; PAGE_SIZE]>,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<UserPageTable>,
    /// The scheduling state of the process.
    pub state: State,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new() -> OsResult<Process> {
        // let stack = Stack::new().ok_or(OsError::NoMemory)?;
        // let mut stack = vmap.alloc(Self::get_stack_base(), PagePerm::RW);
        // for byte in stack.iter_mut() {
        //     *byte = 0;
        // }
        // Ok(Self {
        //     context: Box::new(TrapFrame::default()),
        //     // stack,
        //     // stack: Unique::new(stack as *mut _).expect("non-null"),
        //     vmap: Box::new(UserPageTable::new()),
        //     state: State::Ready,
        // })
        unimplemented!("Process::new()")
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::do_load(pn)?;

        //FIXME: Set trapframe for the process.
        let mut tf = &mut p.context;
        tf.ELR = Self::get_image_base().as_u64();
        tf.SPSR = (SPSR_EL1::M & 0b0000) | SPSR_EL1::F | SPSR_EL1::A | SPSR_EL1::D;
        tf.SP = Self::get_stack_top().as_u64();
        tf.TTBR0 = crate::VMM.get_baddr().as_u64();
        tf.TTBR1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        let mut vmap = Box::new(UserPageTable::new());
        let mut stack = vmap.alloc(Self::get_stack_base(), PagePerm::RW);
        for byte in stack.iter_mut() {
            *byte = 0;
        }
        let entry = (&crate::FILESYSTEM).open(pn)?;
        let mut file = entry.into_file().ok_or(OsError::NoEntry)?;
        let mut image_addr = Self::get_image_base();
        let file_size = file.size() as usize;
        // kprintln!("file size: {}", file_size);
        for i in 0..file_size / PAGE_SIZE {
            // kprintln!("loading page {}", i);
            let mut page = vmap.alloc(image_addr, PagePerm::RWX);
            file.read_exact(&mut page)?;
            image_addr += VirtualAddr::from(PAGE_SIZE);
        }
        if file_size % PAGE_SIZE != 0 {
            // kprintln!("loading last page");
            let mut page = vmap.alloc(image_addr, PagePerm::RWX);
            file.read_exact(&mut page[..file_size % PAGE_SIZE])?;
        }
        Ok(Self {
            context: Box::new(TrapFrame::default()),
            // stack: Unique::new(stack as *mut _).expect("non-null"),
            vmap: vmap,
            state: State::Ready,
        })
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
        VirtualAddr::from(USER_MAX_VA)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
        VirtualAddr::from(align_down(USER_MAX_VA, PAGE_SIZE))
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack.
    pub fn get_stack_top() -> VirtualAddr {
        VirtualAddr::from(align_down(USER_MAX_VA, 16))
    }

    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
        let mut state = mem::replace(&mut self.state, State::Ready);
        match state {
            State::Ready => true,
            State::Waiting(ref mut event_poll_fn) => {
                if event_poll_fn(self) {
                    true
                } else {
                    self.state = state;
                    false
                }
            }
            _ => {
                self.state = state;
                false
            }
        }
    }
}
