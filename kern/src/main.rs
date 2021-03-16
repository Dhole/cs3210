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

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.
use pi::gpio;
use pi::timer;

fn kmain() -> ! {
    let mut pin16 = gpio::Gpio::new(16).into_output();
    let mut pin26 = gpio::Gpio::new(26).into_output();
    let mut pin19 = gpio::Gpio::new(19).into_output();
    let mut pin13 = gpio::Gpio::new(13).into_output();
    let mut pin6 = gpio::Gpio::new(6).into_output();
    let mut pin5 = gpio::Gpio::new(5).into_output();

    let mut pins = [pin16, pin26, pin19, pin13, pin6, pin5];
    let mut i = 0;
    let mut dir = 1i32;

    loop {
        if dir == 1 {
            pins[i].clear();
            pins[i + 1].set();
        } else {
            pins[i - 1].set();
            pins[i].clear();
        }
        timer::spin_sleep(Duration::from_millis(150));
        i = (i as i32 + dir) as usize;
        if i == 0 {
            dir = 1;
        } else if i == 5 {
            dir = -1;
        }
    }
}
