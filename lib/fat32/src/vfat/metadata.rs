use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    // FIXME: Fill me in.
}

// FIXME: Implement `traits::Timestamp` for `Timestamp`.
impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        unimplemented!()
    }

    fn month(&self) -> u8 {
        unimplemented!()
    }

    fn day(&self) -> u8 {
        unimplemented!()
    }

    fn hour(&self) -> u8 {
        unimplemented!()
    }

    fn minute(&self) -> u8 {
        unimplemented!()
    }

    fn second(&self) -> u8 {
        unimplemented!()
    }
}

// FIXME: Implement `traits::Metadata` for `Metadata`.
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        unimplemented!()
    }

    fn hidden(&self) -> bool {
        unimplemented!()
    }

    fn created(&self) -> Self::Timestamp {
        unimplemented!()
    }

    fn accessed(&self) -> Self::Timestamp {
        unimplemented!()
    }

    fn modified(&self) -> Self::Timestamp {
        unimplemented!()
    }
}

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.
