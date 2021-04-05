use core::fmt;
use core::fmt::Debug;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

pub fn split_sector_cylinder(bytes: [u8; 2]) -> (u8, u16) {
    let v = u16::from_le_bytes(bytes);
    ((v & 0b0011_1111) as u8, (v & 0b1111_1111_1100_0000) >> 6)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    head: u8,
    _sector_cylinder: [u8; 2],
}

impl CHS {
    fn sector_cylinder(&self) -> (u8, u16) {
        split_sector_cylinder(self._sector_cylinder)
    }
}

impl Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let (sector, cylinder) = self.sector_cylinder();
        f.debug_struct("CHS")
            .field("head", &self.head)
            .field("sector", &sector)
            .field("cylinder", &cylinder)
            .finish()
    }
}

const_assert_size!(CHS, 3);

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct PartitionEntry {
    boot_indicator: u8,
    starting_chs: CHS,
    partition_type: u8,
    ending_chs: CHS,
    relative_sector: u32,
    total_sectors: u32,
}

impl Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("PartitionEntry")
            .field("boot_indicator", &self.boot_indicator)
            .field("starting_chs", &self.starting_chs)
            .field("partition_type", &self.partition_type)
            .field("ending_chs", &self.ending_chs)
            .field("relative_sector", &self.relative_sector)
            .field("total_sectors", &self.total_sectors)
            .finish()
    }
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    partition_table: [PartitionEntry; 4],
    signature: [u8; 2],
}

impl Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("MasterBootRecord")
            .field("disk_id", &self.disk_id)
            .field("partition_table", &self.partition_table)
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut sector_data = vec![0; device.sector_size() as usize];
        device.read_sector(0, &mut sector_data)?;
        let mbr = unsafe { *{ sector_data.as_ptr() as *const MasterBootRecord } };
        if mbr.signature != [0x55, 0xAA] {
            return Err(Error::BadSignature);
        }
        for (n, part) in mbr.partition_table.iter().enumerate() {
            if part.boot_indicator != 0x00 && part.boot_indicator != 0x80 {
                return Err(Error::UnknownBootIndicator(n as u8));
            }
        }
        Ok(mbr)
    }
}
