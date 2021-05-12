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

// use ::shell::shell_io;

fn kmain() -> ! {
    // let mut console = Console::new();
    // console.initialize();
    // shell_io("> ", console);

    // let (start, end) = memory_map().unwrap();
    // kprintln!("Memory map: 0x{:x}, 0x{:x}", start, end);
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }
    kprintln!("Welcome to cs3210!");
    // kprintln!("Atags:");
    // for atag in Atags::get() {
    //     kprintln!("{:#?}", atag);
    // }

    // use alloc::vec::Vec;
    // // let mut v = Vec::<u8>::new();
    // // for i in 0..50 {
    // //     v.push(i);
    // //     kprintln!("{:?}", v);
    // // }
    // use alloc::vec;
    // kprintln!("Sd::new");
    // let mut sd = unsafe { Sd::new() }.unwrap();
    // let mut buf = vec![0u8; 512];
    // // let mut buf = Vec::<u8>::new();

    // use fat32::traits::BlockDevice;
    // for s in 0..4 {
    //     kprintln!("sd.read_sector");
    //     sd.read_sector(s, &mut buf).unwrap();
    //     for (i, b) in buf.iter().enumerate() {
    //         if i % 16 == 0 {
    //             kprint!("{:04x}: ", i);
    //         }
    //         kprint!("{:02x} ", b);
    //         if i % 16 == 15 {
    //             kprintln!();
    //         }
    //     }
    //     kprintln!();
    // }

    // use fat32::traits::{Dir, Entry, FileSystem};
    // kprintln!("FILESYSTEM.open");
    // let entry = (&FILESYSTEM).open("/").unwrap();
    // kprintln!("entry.as_dir");
    // let root = entry.as_dir().unwrap();
    // kprintln!("root.entries");
    // let mut entries = root.entries().unwrap();
    // // for i in 0..10 {
    // //     kprintln!("{:?}", entries.raw_entries[i]);
    // // }
    // kprintln!("--- BEGIN ---");
    // for e in entries {
    //     kprintln!("{}", e.name());
    // }
    // // let e0 = entries.next().unwrap();
    // // kprintln!("{}", e0.name());
    // kprintln!("--- END ---");
    // shell::shell("> ", &FILESYSTEM);
    kprintln!("current_el: {}", unsafe { current_el() });
    brk!(2);
    // svc!(721);
    kprintln!("after brk");
    loop {}
}
