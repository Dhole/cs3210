#![cfg(feature = "local")]
use shell::shell_io;
use shim::io;
use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW};

struct ReadWrite<R, W> {
    r: R,
    w: W,
}

impl<R: io::Read, W> io::Read for ReadWrite<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.r.read(buf)
    }
}

impl<R, W: io::Write> io::Write for ReadWrite<R, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.w.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

fn main() {
    let stdin = 0;
    let termios = Termios::from_fd(stdin).unwrap();
    let mut new_termios = termios.clone(); // make a mutable copy of termios
                                           // that we will modify
    new_termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(stdin, TCSANOW, &mut new_termios).unwrap();
    let mut rw = ReadWrite {
        r: io::stdin(),
        w: io::stdout(),
    };
    shell_io("> ", &mut rw);
}
