#![feature(decl_macro)]
#![feature(optin_builtin_traits)]
#![no_std]

use fat32::traits::Entry;
use fat32::traits::FileSystem;
use fat32::vfat::Dir;
use fat32::vfat::VFatHandle;
use shim::io;
use shim::io::{Read, Seek};
use shim::path::{Component, Path, PathBuf};
use shim::{ioerr, newioerr};
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

fn cmd_echo<'a, T: io::Read + io::Write, F: FileSystem>(
    args: &StackVec<'a, &'a str>,
    cwd: &mut Cwd<F>,
    rw: &mut T,
) -> io::Result<()> {
    if args.len() == 1 {
        writeln!(rw);
        return Ok(());
    }
    write!(rw, "{}", args[1]);
    if args.len() > 2 {
        for arg in &args.as_slice()[2..] {
            write!(rw, " {}", arg);
        }
    }
    writeln!(rw);
    Ok(())
}

struct Cwd<F: FileSystem> {
    fs: F,
    // dir: F::Dir,
    path: PathBuf,
}

impl<F: FileSystem> Cwd<F> {
    fn new(fs: F) -> Self {
        let root_path = Path::new("/").to_path_buf();
        // let entry = fs.open(root_path).expect("open FileSystem root");
        // let root = entry.into_dir().expect("root entry into dir");
        Self {
            fs: fs,
            // dir: root,
            path: root_path,
        }
    }

    fn resolve_path(&self, ext: &str) -> io::Result<PathBuf> {
        let ext_path = Path::new(ext);
        let mut new_path = self.path.clone();
        for component in ext_path.components() {
            match component {
                Component::Prefix(pre) => {
                    return ioerr!(InvalidInput, "unpexpected prefix in path")
                }
                Component::RootDir => {
                    new_path = Path::new("/").to_path_buf();
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    new_path.pop();
                }
                Component::Normal(comp) => {
                    new_path.push(comp);
                }
            }
        }
        Ok(new_path)
    }

    fn cd(&mut self, path: &PathBuf) -> io::Result<()> {
        match self.fs.open_dir(path) {
            Ok(_) => {
                self.path = path.clone();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

fn cmd_pwd<'a, T: io::Read + io::Write, F: FileSystem>(
    args: &StackVec<'a, &'a str>,
    cwd: &mut Cwd<F>,
    rw: &mut T,
) -> io::Result<()> {
    let path = cwd
        .path
        .to_str()
        .ok_or(newioerr!(InvalidData, "path is not valid utf-8"))?;
    writeln!(rw, "{}", path);
    Ok(())
}

fn cmd_cd<'a, T: io::Read + io::Write, F: FileSystem>(
    args: &StackVec<'a, &'a str>,
    cwd: &mut Cwd<F>,
    rw: &mut T,
) -> io::Result<()> {
    if args.len() < 2 {
        return ioerr!(InvalidInput, "no path provided");
    }
    let new_path = cwd.resolve_path(args[1])?;
    cwd.cd(&new_path)
}

fn cmd_cat<'a, T: io::Read + io::Write, F: FileSystem>(
    args: &StackVec<'a, &'a str>,
    cwd: &mut Cwd<F>,
    rw: &mut T,
) -> io::Result<()> {
    if args.len() < 2 {
        return ioerr!(InvalidInput, "no path provided");
    }
    let path = cwd.resolve_path(args[1])?;
    let entry = cwd.fs.open(path)?;
    let mut file = entry
        .into_file()
        .ok_or(newioerr!(InvalidInput, "path is not a file"))?;
    let mut buf = [0u8; 512];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let text = &buf[..n];
        match core::str::from_utf8(text) {
            Ok(s) => writeln!(rw, "{}", s),
            Err(e) => {
                let end = e.valid_up_to();
                if end == 0 {
                    return ioerr!(InvalidData, "non utf-8 data");
                }
                // This is safe due to the above check
                let s = unsafe { core::str::from_utf8_unchecked(&text[..end]) };
                writeln!(rw, "{}", s);
                let offset = (end - n) as i64;
                file.seek(io::SeekFrom::Current(-1 * offset))?;
            }
        }
    }
    Ok(())
}

fn cmd_ls<'a, T: io::Read + io::Write, F: FileSystem>(
    args: &StackVec<'a, &'a str>,
    cwd: &mut Cwd<F>,
    rw: &mut T,
) -> io::Result<()> {
    use fat32::traits::Dir;
    use fat32::traits::Metadata;

    let mut args = &args.as_slice()[1..];
    let show_hidden = if args.len() > 0 && args[0] == "-a" {
        args = &args[1..];
        true
    } else {
        false
    };
    let arg_path = match args.len() {
        0 => ".",
        1 => args[0],
        _ => return ioerr!(InvalidInput, "too many arguments"),
    };
    let path = cwd.resolve_path(arg_path)?;
    let dir = cwd.fs.open_dir(path)?;
    for entry in dir.entries()? {
        let metadata = entry.metadata();
        if !show_hidden & metadata.hidden() {
            continue;
        }
        writeln!(
            rw,
            "{dir}{file}{read_only}{hidden} {modified:?} {name}",
            dir = if entry.is_dir() { "d" } else { "-" },
            file = if entry.is_file() { "f" } else { "-" },
            read_only = if metadata.read_only() { "r" } else { "-" },
            hidden = if metadata.hidden() { "h" } else { "-" },
            modified = metadata.modified(),
            name = entry.name(),
        );
    }
    Ok(())
}

fn run<T: io::Read + io::Write, F: FileSystem>(
    cmd: &Command,
    cwd: &mut Cwd<F>,
    rw: &mut T,
) -> io::Result<()> {
    let path = cmd.path();
    let res = match path {
        "echo" => cmd_echo(&cmd.args, cwd, rw),
        "pwd" => cmd_pwd(&cmd.args, cwd, rw),
        "cd" => cmd_cd(&cmd.args, cwd, rw),
        "ls" => cmd_ls(&cmd.args, cwd, rw),
        "cat" => cmd_cat(&cmd.args, cwd, rw),
        unk => {
            writeln!(rw, "ERR: unknown command: {}", path);
            Ok(())
        }
    };
    rw.flush().unwrap();
    res
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell_io<T: io::Read + io::Write, F: FileSystem>(prefix: &str, mut rw: T, fs: F) -> ! {
    let mut buffer = [0u8; 512];
    let mut input = StackVec::new(&mut buffer);
    let mut cwd = Cwd::new(fs);
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
                        Ok(ref cmd) => match run(cmd, &mut cwd, &mut rw) {
                            Ok(_) => {}
                            Err(e) => {
                                writeln!(rw, "ERR: Command error: {:?}", e);
                            }
                        },
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
