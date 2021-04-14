#![feature(asm)]
#![feature(global_asm)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

use core::time::Duration;
use pi::gpio::{Gpio, Output};
use shim::io::Write;
use xmodem::{self, Xmodem};

/// Start address of the binary to load and of the bootloader.
const BINARY_START_ADDR: usize = 0x100000;
const BOOTLOADER_START_ADDR: usize = 0x4000000;

/// Pointer to where the loaded binary expects to be laoded.
const BINARY_START: *mut u8 = BINARY_START_ADDR as *mut u8;

/// Free space between the bootloader and the loaded binary's start address.
const MAX_BINARY_SIZE: usize = BOOTLOADER_START_ADDR - BINARY_START_ADDR;

/// Branches to the address `addr` unconditionally.
unsafe fn jump_to(addr: *mut u8) -> ! {
    asm!("br $0" : : "r"(addr as usize));
    loop {
        asm!("wfe" :::: "volatile")
    }
}

fn progress_fn(p: xmodem::Progress) {
    let mut pin16 = Gpio::new(16).into_output();
    if let xmodem::Progress::Packet(n) = p {
        if n % 2 == 0 {
            pin16.set();
        } else {
            pin16.clear();
        }
    }
}

fn blink(pin: &mut Gpio<Output>, times: usize, dur: u64) {
    for _ in 0..times {
        pin.set();
        pi::timer::spin_sleep(Duration::from_millis(dur));
        pin.clear();
        pi::timer::spin_sleep(Duration::from_millis(dur));
    }
}

fn kmain() -> ! {
    let mut pin16 = pi::gpio::Gpio::new(16).into_output();

    // Flash LED fast to indicate power ON
    blink(&mut pin16, 4, 50);
    let mut uart = pi::uart::MiniUart::new();
    uart.set_read_timeout(Duration::from_millis(500));

    xmodem::wait_msg(&mut uart, "HELLO\n");
    uart.write_all("HELLO\n".as_bytes()).unwrap();
    loop {
        let binary_buf = unsafe { core::slice::from_raw_parts_mut(BINARY_START, MAX_BINARY_SIZE) };
        match Xmodem::receive_with_progress(&mut uart, binary_buf, progress_fn) {
            Ok(_) => {
                blink(&mut pin16, 4, 100);
                unsafe { jump_to(BINARY_START) }
            }
            Err(_e) => {
                // let mut uart2 = pi::uart::MiniUart::new();
                // write!(uart2, "E: {:?}\n", _e).unwrap();
                // Blink LED slow to indicate error (in general, timeout)
                blink(&mut pin16, 1, 400)
            }
        }
    }
}
