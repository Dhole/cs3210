use alloc::boxed::Box;
use core::time::Duration;

use crate::console::CONSOLE;
use crate::process::State;
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;
use pi::timer::current_time;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    let time_sleep = current_time();
    SCHEDULER.switch(
        State::Waiting(Box::new(move |p| {
            let now = current_time();
            let elapsed = (now - time_sleep).as_millis() as u32;
            if elapsed >= ms {
                p.context.x[0] = elapsed as u64;
                p.context.x[7] = 1;
                true
            } else {
                false
            }
        })),
        tf,
    );
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {
    let time = current_time();
    tf.x[0] = time.as_secs();
    tf.x[1] = time.subsec_nanos() as u64;
    tf.x[7] = 1;
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    SCHEDULER.kill(tf);
    SCHEDULER.switch_to(tf);
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    use shim::io::Write;

    (&crate::console::CONSOLE).write(&[b]);
    tf.x[7] = 1;
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    tf.x[0] = tf.TPIDR;
    tf.x[7] = 1;
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    use crate::console::kprintln;
    match num as usize {
        NR_SLEEP => {
            sys_sleep(tf.x[0] as u32, tf);
        }
        NR_TIME => {
            sys_time(tf);
        }
        NR_EXIT => {
            sys_exit(tf);
        }
        NR_WRITE => {
            sys_write(tf.x[0] as u8, tf);
        }
        NR_GETPID => {
            sys_getpid(tf);
        }
        _ => {
            unimplemented!("syscall {}", num);
        }
    }
}
