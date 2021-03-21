#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

pub mod console;
pub mod mutex;
pub mod shell;

// use console::Console;

// use ::shell::shell_io;

fn kmain() -> ! {
    // let mut console = Console::new();
    // console.initialize();
    // shell_io("> ", console);
    shell::shell("> ");
}
