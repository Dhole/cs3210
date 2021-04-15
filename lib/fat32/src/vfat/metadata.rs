use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

impl Date {
    pub fn day(&self) -> u8 {
        (self.0 & 0b0000_0000__0001_1111) as u8
    }
    pub fn month(&self) -> u8 {
        ((self.0 & 0b0000_0001__1110_0000) >> 5) as u8
    }
    pub fn year(&self) -> usize {
        1980 + ((self.0 & 0b1111_1110__0000_0000) >> 9) as usize
    }
}

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

impl Time {
    pub fn from(t: u16) -> Self {
        Self(t)
    }
    pub fn second(&self) -> u8 {
        ((self.0 & 0b0000_0000__0001_1111) * 2) as u8
    }
    pub fn minute(&self) -> u8 {
        ((self.0 & 0b0000_0111__1110_0000) >> 5) as u8
    }
    pub fn hour(&self) -> u8 {
        ((self.0 & 0b1111_1000__0000_0000) >> 11) as u8
    }
}

const ATTR_READ_ONLY: u8 = 1 << 0;
const ATTR_HIDDEN: u8 = 1 << 1;
const ATTR_SYSTEM: u8 = 1 << 2;
const ATTR_VOLUME_ID: u8 = 1 << 3;
const ATTR_DIRECTORY: u8 = 1 << 4;
const ATTR_ARCHIVE: u8 = 1 << 5;
const ATTR_LFN: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

const ROOTDIR_ATTRIBUTES: Attributes = Attributes(ATTR_DIRECTORY);

impl Attributes {
    pub fn raw(&self) -> u8 {
        self.0
    }
    pub fn read_only(&self) -> bool {
        self.0 & ATTR_READ_ONLY != 0
    }
    pub fn hidden(&self) -> bool {
        self.0 & ATTR_HIDDEN != 0
    }
    pub fn system(&self) -> bool {
        self.0 & ATTR_SYSTEM != 0
    }
    pub fn volume_id(&self) -> bool {
        self.0 & ATTR_VOLUME_ID != 0
    }
    pub fn directory(&self) -> bool {
        self.0 & ATTR_DIRECTORY != 0
    }
    pub fn archive(&self) -> bool {
        self.0 & ATTR_ARCHIVE != 0
    }
    pub fn lfn(&self) -> bool {
        self.0 & ATTR_LFN == ATTR_LFN
    }
}

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

use core::fmt::Debug;

impl Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use crate::traits::Timestamp;
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year(),
            self.month(),
            self.day(),
            self.hour(),
            self.minute(),
            self.second()
        )
    }
}

const ROOTDIR_TIMESTAMP: Timestamp = Timestamp {
    date: Date(0),
    time: Time(0),
};

/// Metadata for a directory entry.
#[derive(Default, Clone)]
pub struct Metadata {
    pub attributes: Attributes,
    pub created_ts: Timestamp,
    pub modified_ts: Timestamp,
    pub accessed_ts: Timestamp,
}

pub const ROOTDIR_METADATA: Metadata = Metadata {
    attributes: ROOTDIR_ATTRIBUTES,
    created_ts: ROOTDIR_TIMESTAMP,
    modified_ts: ROOTDIR_TIMESTAMP,
    accessed_ts: ROOTDIR_TIMESTAMP,
};

impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        self.date.year()
    }

    fn month(&self) -> u8 {
        self.date.month()
    }

    fn day(&self) -> u8 {
        self.date.day()
    }

    fn hour(&self) -> u8 {
        self.time.hour()
    }

    fn minute(&self) -> u8 {
        self.time.minute()
    }

    fn second(&self) -> u8 {
        self.time.second()
    }
}

impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attributes.read_only()
    }

    fn hidden(&self) -> bool {
        self.attributes.hidden()
    }

    fn created(&self) -> Self::Timestamp {
        self.created_ts
    }

    fn accessed(&self) -> Self::Timestamp {
        self.accessed_ts
    }

    fn modified(&self) -> Self::Timestamp {
        self.modified_ts
    }
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use traits::Metadata;

        f.debug_struct("Metadata")
            .field("read_only", &self.read_only())
            .field("hidden", &self.hidden())
            .field("created", &self.created())
            .field("accessed", &self.accessed())
            .field("modified", &self.modified())
            .finish()
    }
}
