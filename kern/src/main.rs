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
use pi::timer;

unsafe fn kmain() -> ! {
    const GPIO_BASE: usize = 0x3F000000 + 0x200000;

    const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
    const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
    const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

    // STEP 1: Set GPIO Pin 16 as output.
    GPIO_FSEL1
        .write_volatile((GPIO_FSEL1.read_volatile() & !(0b111 << (6 * 3))) | (0b001 << (6 * 3)));
    // STEP 2: Continuously set and clear GPIO 16.
    loop {
        GPIO_SET0.write_volatile(0b1 << 16);
        timer::spin_sleep(Duration::from_secs(1));
        GPIO_CLR0.write_volatile(0b1 << 16);
        timer::spin_sleep(Duration::from_secs(1));
    }
}
