use crate::console::{kprint, kprintln, CONSOLE};
use core::fmt::Write;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!(
        r#"
    ████████
   █        █
  █  X    X  █
  █   ____   █
  █  /    \  █
   █        █
    ████████

     PANIC!
"#
    );
    if let Some(location) = info.location() {
        kprintln!(
            "At {}:{},{}",
            location.file(),
            location.line(),
            location.column(),
        );
    } else {
        kprintln!("At unknown location");
    }
    if let Some(message) = info.message() {
        let mut console = CONSOLE.lock();
        console.write_fmt(*message).unwrap();
        kprintln!();
    }

    loop {}
}
