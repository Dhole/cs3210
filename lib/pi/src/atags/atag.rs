use crate::atags::raw;

pub use crate::atags::raw::{Core, Mem};

/// An ATAG.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Atag {
    Core(raw::Core),
    Mem(raw::Mem),
    Cmd(&'static str),
    Unknown(u32),
    None,
}

impl Atag {
    /// Returns `Some` if this is a `Core` ATAG. Otherwise returns `None`.
    pub fn core(self) -> Option<Core> {
        if let Atag::Core(core) = self {
            Some(core)
        } else {
            None
        }
    }

    /// Returns `Some` if this is a `Mem` ATAG. Otherwise returns `None`.
    pub fn mem(self) -> Option<Mem> {
        if let Atag::Mem(mem) = self {
            Some(mem)
        } else {
            None
        }
    }

    /// Returns `Some` with the command line string if this is a `Cmd` ATAG.
    /// Otherwise returns `None`.
    pub fn cmd(self) -> Option<&'static str> {
        if let Atag::Cmd(cmd) = self {
            Some(cmd)
        } else {
            None
        }
    }
}

impl From<&'static raw::Atag> for Atag {
    fn from(atag: &'static raw::Atag) -> Atag {
        unsafe {
            match (atag.tag, &atag.kind) {
                (raw::Atag::CORE, &raw::Kind { core }) => Atag::Core(core),
                (raw::Atag::MEM, &raw::Kind { mem }) => Atag::Mem(mem),
                (raw::Atag::CMDLINE, &raw::Kind { ref cmd }) => {
                    let start = &cmd.cmd as *const u8;
                    let mut ptr = &cmd.cmd as *const u8;
                    while *ptr != 0 {
                        ptr = ptr.add(1);
                    }
                    let c_str_len = ptr.offset_from(start) as usize;
                    let c_str_slice = core::slice::from_raw_parts(start, c_str_len);
                    let cmd_str = core::str::from_utf8(c_str_slice).expect("valid utf8");
                    Atag::Cmd(cmd_str)
                }
                (raw::Atag::NONE, _) => Atag::None,
                (id, _) => Atag::Unknown(id),
            }
        }
    }
}
