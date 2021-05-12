use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub ELR: u64,
    pub SPSR: u64,
    pub SP: u64,
    pub TPIDR: u64,
    pub q: [u128; 32],
    pub x: [u64; 30],
    pub lr: u64,
    _xzr: u64,
}
