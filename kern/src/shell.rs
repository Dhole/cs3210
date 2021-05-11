// use stack_vec::StackVec;
use shell::shell_io;

// use crate::console::{kprint, kprintln, CONSOLE};
use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;

use pi::atags::Atags;

/*
use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry};
*/

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
// use crate::FILESYSTEM;

// /// Error type for `Command` parse failures.
// #[derive(Debug)]
// enum Error {
//     Empty,
//     TooManyArgs,
// }
//
// /// A structure representing a single shell command.
// struct Command<'a> {
//     args: StackVec<'a, &'a str>,
// }
//
// impl<'a> Command<'a> {
//     /// Parse a command from a string `s` using `buf` as storage for the
//     /// arguments.
//     ///
//     /// # Errors
//     ///
//     /// If `s` contains no arguments, returns `Error::Empty`. If there are more
//     /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
//     fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
//         let mut args = StackVec::new(buf);
//         for arg in s.split(' ').filter(|a| !a.is_empty()) {
//             args.push(arg).map_err(|_| Error::TooManyArgs)?;
//         }
//
//         if args.is_empty() {
//             return Err(Error::Empty);
//         }
//
//         Ok(Command { args })
//     }
//
//     /// Returns this command's path. This is equivalent to the first argument.
//     fn path(&self) -> &str {
//         self.args[0]
//     }
// }
//
// /// Starts a shell using `prefix` as the prefix for each line. This function
// /// returns if the `exit` command is called.
// pub fn shell(prefix: &str) -> ! {
//     use core::fmt::{self, Write};
//
//     let mut console = CONSOLE.lock();
//     loop {
//         console.write_str(prefix).unwrap();
//         loop {
//             let b = console.read_byte();
//             match b {
//                 b'\r' => {}
//                 b'\n' => {} // Return
//                 8 | 127 => {
//                     console.write_byte(8);
//                     console.write_byte(b' ');
//                     console.write_byte(8);
//                 } // backspace
//                 0x20..=0x7E => {}
//                 _ => {}
//             }
//         }
//     }
// }

use fat32::traits::FileSystem;

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell<F: FileSystem>(prefix: &str, fs: F) {
    shell_io(prefix, &CONSOLE, fs)
}
