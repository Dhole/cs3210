#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod param;
pub mod process;
pub mod shell;
pub mod traps;
pub mod vm;

use aarch64::{brk, current_el, svc};
use allocator::memory_map;
use allocator::Allocator;
use console::{kprint, kprintln};
use fs::sd::Sd;
use fs::FileSystem;
use pi::atags::Atags;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();

fn kmain() -> ! {
    kprintln!("Welcome to cs3210!");
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
        // SCHEDULER.initialize();
        IRQ.initialize();

        SCHEDULER.start();
    }
    loop {}
}
