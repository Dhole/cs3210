#![feature(decl_macro)]
#![feature(optin_builtin_traits)]
#![no_std]

use shim::io;
use stack_vec::StackVec;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

macro writeln {
    ($rw:expr) => (write!($rw, "\n")),
    ($rw:expr, $fmt:expr) => (write!($rw, concat!($fmt, "\n"))),
    ($rw:expr, $fmt:expr, $($arg:tt)*) => (write!($rw, concat!($fmt, "\n"), $($arg)*))
}

macro write($rw:expr, $($arg:tt)*) {
    core::write!($rw, $($arg)*).unwrap()
}

fn run<T: io::Read + io::Write>(cmd: &Command, rw: &mut T) {
    let path = cmd.path();
    if path == "echo" {
        if cmd.args.len() == 1 {
            writeln!(rw);
            return;
        }
        write!(rw, "{}", cmd.args[1]);
        if cmd.args.len() > 2 {
            for arg in &cmd.args.as_slice()[2..] {
                write!(rw, " {}", arg);
            }
        }
        writeln!(rw);
    } else {
        writeln!(rw, "ERR: unknown command: {}", path);
    }
    rw.flush().unwrap();
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell_io<T: io::Read + io::Write>(prefix: &str, mut rw: T) -> ! {
    let mut buffer = [0u8; 512];
    let mut input = StackVec::new(&mut buffer);
    'prompt: loop {
        input.truncate(0);
        rw.write(prefix.as_bytes()).unwrap();
        rw.flush().unwrap();
        loop {
            let mut _b = [0];
            rw.read_exact(&mut _b).unwrap();
            let b = _b[0];
            match b {
                b'\r' | b'\n' => {
                    // Return
                    writeln!(rw);
                    rw.flush().unwrap();
                    let s = match core::str::from_utf8(input.as_slice()) {
                        Ok(s) => s,
                        Err(e) => {
                            writeln!(rw, "ERR: str::from_utf8: {:?}", e);
                            continue 'prompt;
                        }
                    };
                    let mut cmd_buf = [""; 64];
                    match Command::parse(s, &mut cmd_buf) {
                        Ok(ref cmd) => run(cmd, &mut rw),
                        Err(Error::Empty) => {}
                        Err(Error::TooManyArgs) => {
                            writeln!(rw, "ERR: Too many args");
                        }
                    }
                    continue 'prompt;
                }
                8 | 127 => {
                    // Backspace
                    if !input.is_empty() {
                        input.pop();
                        write!(rw, "\u{8} \u{8}");
                    }
                }
                0x20..=0x7E => {
                    // Visible character
                    if input.push(b).is_ok() {
                        rw.write(&[b]).unwrap();
                    }
                }
                _ => {
                    // Non-visible character
                    write!(rw, "\u{7}");
                }
            }
            rw.flush().unwrap();
        }
    }
}
