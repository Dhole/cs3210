use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::entry::EntryValue;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};

#[derive(Debug, Clone)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub first_cluster: Cluster,
    // FIXME: Fill me in.
}

pub struct DirIter<HANDLE: VFatHandle> {
    dir: Dir<HANDLE>,
    raw_entries: Vec<VFatDirEntry>,
    pos: usize,
}

impl<HANDLE: VFatHandle> DirIter<HANDLE> {
    pub fn entry_name(&self, regular_entry: &VFatRegularDirEntry, pos: usize) -> (String, usize) {
        if !regular_entry.attributes.lfn() {
            regular_entry.file_name
        } else {
        }
    }
}

impl<HANDLE: VFatHandle> Iterator for DirIter<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        let regular_entry = unsafe { &self.raw_entries[self.pos].regular };
        let first_cluster = regular_entry.first_cluster();
        let value = if regular_entry.attributes.directory() {
            EntryValue::Dir(Dir {
                vfat: self.dir.vfat.clone(),
                first_cluster: first_cluster,
            })
        } else {
            EntryValue::File(File {
                vfat: self.dir.vfat.clone(),
                first_cluster: first_cluster,
            })
        };
        let metadata = regular_entry.metadata();
        let (name, new_pos) = self.entry_name(&regular_entry, self.pos);
        self.pos = new_pos;
        Some(Entry {
            value,
            _metadata: metadata,
            _name: name,
        })
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    file_name: [u8; 8],
    file_ext: [u8; 3],
    attributes: Attributes,
    reserved_winnt: u8,
    created_time_secs: u8,
    created_time: Time,
    created_date: Date,
    accessed_date: Date,
    first_cluster_hi: u16,
    modified_time: Time,
    modified_date: Date,
    first_cluster_lo: u16,
    size: u32,
}

impl VFatRegularDirEntry {
    pub fn first_cluster(&self) -> Cluster {
        Cluster::from(self.first_cluster_lo as u32 | (self.first_cluster_hi as u32) << 16)
    }
    pub fn metadata(&self) -> Metadata {
        Metadata {
            attributes: self.attributes,
            created_ts: Timestamp {
                date: self.created_date,
                time: self.created_time,
            },
            modified_ts: Timestamp {
                date: self.modified_date,
                time: self.modified_time,
            },
            accessed_ts: Timestamp {
                date: self.accessed_date,
                time: Time::from(0),
            },
        }
    }
    pub fn name(&self) -> String {
        unimplemented!()
    }
    pub fn entry<HANDLE: VFatHandle>(&self) -> Entry<HANDLE> {
        unimplemented!()
    }
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    seq_num: u8,
    name0: [u16; 5],
    attributes: Attributes,
    entry_type: u8,
    name_checksum: u8,
    name1: [u16; 6],
    zeroes: u16,
    name2: [u16; 2],
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    id: u8,
    reserved0: [u8; 9],
    attributes: Attributes,
    reserved1: [u8; 21],
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        unimplemented!("Dir::find()")
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    type Entry = Entry<HANDLE>;
    type Iter = DirIter<HANDLE>;

    fn entries(&self) -> io::Result<Self::Iter> {
        let mut data = Vec::new();
        self.vfat.lock(|vfat| -> io::Result<()> {
            vfat.read_chain(self.first_cluster, &mut data)?;
            Ok(())
        })?;
        Ok(DirIter {
            dir: self.clone(),
            raw_entries: unsafe { data.cast() },
            pos: 0,
        })
    }
}
