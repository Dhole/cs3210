use alloc::string::String;
use alloc::string::ToString;
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
}

pub struct DirIter<HANDLE: VFatHandle> {
    dir: Dir<HANDLE>,
    pub raw_entries: Vec<VFatDirEntry>,
    pos: usize,
}

fn regular_entry_name(regular_entry: &VFatRegularDirEntry) -> String {
    let file_name_len = regular_entry
        .file_name
        .iter()
        .position(|&b| b == 0x00 || b == 0x20)
        .unwrap_or(regular_entry.file_name.len());
    let file_ext_len = regular_entry
        .file_ext
        .iter()
        .position(|&b| b == 0x00 || b == 0x20)
        .unwrap_or(regular_entry.file_ext.len());
    let mut name = String::from_utf8_lossy(&regular_entry.file_name[..file_name_len]).to_string();
    if file_ext_len != 0 {
        let ext = String::from_utf8_lossy(&regular_entry.file_ext[..file_ext_len]).to_string();
        name += ".";
        name += &ext;
    }
    name.to_string()
}

const MAX_LFN_ENTRIES: usize = 0x14;
const LFN_ENTRY_LEN: usize = 13;

impl<HANDLE: VFatHandle> DirIter<HANDLE> {
    pub fn entry_name(&self, raw_entry: &VFatDirEntry, mut pos: usize) -> (String, usize) {
        let unknown_entry = unsafe { &raw_entry.unknown };
        if !unknown_entry.attributes.lfn() {
            let regular_entry = unsafe { &raw_entry.regular };
            (regular_entry_name(regular_entry), pos)
        } else {
            let mut name_u16 = vec![0xffffu16; MAX_LFN_ENTRIES * LFN_ENTRY_LEN];
            let mut raw_name = vec![0u16; LFN_ENTRY_LEN];
            loop {
                let raw_entry = &self.raw_entries[pos];
                let unknown_entry = unsafe { &raw_entry.unknown };
                if !unknown_entry.attributes.lfn() {
                    break;
                }
                pos += 1;
                let lfn_entry = unsafe { &raw_entry.long_filename };
                let seq_num = ((lfn_entry.seq_num & 0x1f) - 1) as usize;
                // if seq_num == 0 {
                //     break;
                // }
                raw_name[0..5].copy_from_slice(&lfn_entry.name0);
                raw_name[5..11].copy_from_slice(&lfn_entry.name1);
                raw_name[11..13].copy_from_slice(&lfn_entry.name2);
                assert!(seq_num < MAX_LFN_ENTRIES);
                name_u16[seq_num * LFN_ENTRY_LEN..seq_num * LFN_ENTRY_LEN + LFN_ENTRY_LEN]
                    .copy_from_slice(&raw_name[..]);
            }
            let name_len = name_u16
                .iter()
                .position(|&b| b == 0x0000 || b == 0xffff)
                .unwrap_or(name_u16.len());
            let name = String::from_utf16_lossy(&name_u16[..name_len]);
            (name, pos)
        }
    }
}

impl<HANDLE: VFatHandle> Iterator for DirIter<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut raw_entry = &self.raw_entries[self.pos];
        loop {
            let unknown_entry = unsafe { raw_entry.unknown };
            match unknown_entry.id {
                0x00 => return None,
                0xE5 => {}
                _ => {
                    break;
                }
            }
            self.pos += 1;
            raw_entry = &self.raw_entries[self.pos];
        }
        let (name, new_pos) = self.entry_name(&raw_entry, self.pos);
        self.pos = new_pos;
        let mut raw_entry = &self.raw_entries[self.pos];
        let regular_entry = unsafe { raw_entry.regular };
        let first_cluster = regular_entry.first_cluster();
        let value = if regular_entry.attributes.directory() {
            EntryValue::Dir(Dir {
                vfat: self.dir.vfat.clone(),
                first_cluster: first_cluster,
            })
        } else {
            EntryValue::File(File::new(
                self.dir.vfat.clone(),
                first_cluster,
                regular_entry.size,
            ))
        };
        let metadata = regular_entry.metadata();
        self.pos += 1;
        Some(Entry {
            value,
            _metadata: metadata,
            _name: name,
        })
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
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
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
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
#[derive(Copy, Clone, Debug)]
pub struct VFatUnknownDirEntry {
    id: u8,
    reserved0: [u8; 10],
    attributes: Attributes,
    reserved1: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

#[derive(Copy, Clone)]
pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

use core::fmt;
use core::fmt::Debug;

impl Debug for VFatDirEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let unknown_entry = unsafe { &self.unknown };
        if unknown_entry.attributes.lfn() {
            let lfn_entry = unsafe { &self.long_filename };
            write!(f, "{:?}", lfn_entry)
        } else {
            let regular_entry = unsafe { &self.regular };
            write!(f, "{:?}", regular_entry)
        }
    }
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
        use traits::Dir;
        use traits::Entry;
        let name = name
            .as_ref()
            .to_str()
            .ok_or(newioerr!(InvalidInput, "name is not utf-8"))?;
        self.entries()?
            .find(|e| e.name().eq_ignore_ascii_case(name))
            .ok_or(newioerr!(NotFound, "file name not found"))
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
        // let raw_entry_len = core::mem::size_of::<VFatDirEntry>();
        // let raw_entries_len = data.len() / raw_entry_len;
        // let mut raw_entries: Vec<VFatDirEntry> = Vec::with_capacity(raw_entries_len);
        // for i in 0..raw_entries_len {
        //     raw_entries
        //         .push(unsafe { *(data[i * raw_entry_len..].as_ptr() as *const VFatDirEntry) });
        // }
        Ok(DirIter {
            dir: self.clone(),
            raw_entries: unsafe { data.cast() },
            // raw_entries,
            pos: 0,
        })
    }
}
