#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

pub mod allocator;
pub mod console;
// pub mod fs;
pub mod mutex;
pub mod shell;

use allocator::memory_map;
use allocator::Allocator;
use console::kprintln;
use pi::atags::Atags;
// use fs::FileSystem;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
// pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

// use ::shell::shell_io;

fn kmain() -> ! {
    // let mut console = Console::new();
    // console.initialize();
    // shell_io("> ", console);

    let (start, end) = memory_map().unwrap();
    kprintln!("Memory map: 0x{:x}, 0x{:x}", start, end);
    unsafe {
        ALLOCATOR.initialize();
        // FILESYSTEM.initialize();
    }
    kprintln!("Welcome to cs3210!");
    kprintln!("Atags:");
    for atag in Atags::get() {
        kprintln!("{:#?}", atag);
    }

    shell::shell("> ");
}
