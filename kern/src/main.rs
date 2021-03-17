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

use console::kprintln;
use core::time::Duration;

use pi::gpio;
use pi::timer::{self, spin_sleep};
use pi::uart;

fn kmain() -> ! {
    let mut uart = uart::MiniUart::new();
    let mut gpio5 = gpio::Gpio::new(5).into_output();
    let mut gpio26 = gpio::Gpio::new(26).into_output();
    let mut flip = false;
    loop {
        // uart.write_byte(0x41);
        let b = uart.read_byte();
        uart.write_byte(b);
        // spin_sleep(Duration::from_millis(200));
        if flip {
            gpio5.set();
            gpio26.clear();
        } else {
            gpio5.clear();
            gpio26.set();
        }
        flip = !flip;
    }
}
