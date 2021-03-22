mod parsers;

use serial;
use serial::prelude::*;
use structopt;
use structopt_derive::StructOpt;
use xmodem::{Progress, Xmodem};

use std::path::PathBuf;
use std::time::Duration;

use serial::core::{BaudRate, CharSize, FlowControl, StopBits};
use structopt::StructOpt;

use parsers::{parse_baud_rate, parse_flow_control, parse_stop_bits, parse_width};

#[derive(StructOpt, Debug)]
#[structopt(about = "Write to TTY using the XMODEM protocol by default.")]
struct Opt {
    #[structopt(
        short = "i",
        help = "Input file (defaults to stdin if not set)",
        parse(from_os_str)
    )]
    input: Option<PathBuf>,

    #[structopt(
        short = "b",
        long = "baud",
        parse(try_from_str = "parse_baud_rate"),
        help = "Set baud rate",
        default_value = "115200"
    )]
    baud_rate: BaudRate,

    #[structopt(
        short = "t",
        long = "timeout",
        parse(try_from_str),
        help = "Set timeout in seconds",
        default_value = "10"
    )]
    timeout: u64,

    #[structopt(
        short = "w",
        long = "width",
        parse(try_from_str = "parse_width"),
        help = "Set data character width in bits",
        default_value = "8"
    )]
    char_width: CharSize,

    #[structopt(help = "Path to TTY device", parse(from_os_str))]
    tty_path: PathBuf,

    #[structopt(
        short = "f",
        long = "flow-control",
        parse(try_from_str = "parse_flow_control"),
        help = "Enable flow control ('hardware' or 'software')",
        default_value = "none"
    )]
    flow_control: FlowControl,

    #[structopt(
        short = "s",
        long = "stop-bits",
        parse(try_from_str = "parse_stop_bits"),
        help = "Set number of stop bits",
        default_value = "1"
    )]
    stop_bits: StopBits,

    #[structopt(short = "r", long = "raw", help = "Disable XMODEM")]
    raw: bool,
}

fn progress_fn(progress: Progress) {
    println!("+ xmodem progress: {:?}", progress);
}

fn main() {
    use std::fs::File;
    use std::io;

    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");
    port.configure(&serial::PortSettings {
        baud_rate: opt.baud_rate,
        char_size: opt.char_width,
        parity: serial::ParityNone,
        stop_bits: opt.stop_bits,
        flow_control: opt.flow_control,
    })
    .expect("configure port");
    port.set_timeout(Duration::from_secs(opt.timeout))
        .expect("set timeout");

    let mut input: Box<dyn io::Read> = match opt.input {
        None => Box::new(io::stdin()),
        Some(path) => Box::new(File::open(path).expect("unable to open input file")),
    };

    let n = if opt.raw {
        io::copy(&mut input, &mut port).expect("copy input to output") as usize
    } else {
        Xmodem::transmit_with_progress(&mut input, &mut port, progress_fn).expect("xmodem transmit")
    };
    println!("Wrote {} bytes to output", n);
}
