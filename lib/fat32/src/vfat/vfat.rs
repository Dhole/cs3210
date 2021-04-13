use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::string::String;
use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

// use crate::util::print_hex;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::cache::read_n_sectors;
use crate::vfat::entry::EntryValue;
use crate::vfat::metadata::ROOTDIR_METADATA;
use crate::vfat::{BiosParameterBlock, BlockDeviceCached, BlockDevicePartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};

#[derive(Debug)]
pub struct Chain<HANDLE: VFatHandle> {
    vfat: HANDLE,
    next: Option<Cluster>,
}

impl<HANDLE: VFatHandle> Iterator for Chain<HANDLE> {
    type Item = io::Result<Cluster>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.next {
            None => return None,
            Some(next) => next,
        };
        self.next = match self.vfat.lock(|vfat| Ok(vfat.fat_entry(next)?.status())) {
            Ok(status) => match status {
                Status::Data(cluster) => Some(cluster),
                Status::Eoc(_) => None,
                status => return Some(ioerr!(InvalidData, "Invalid chain fat entry")),
            },
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(next))
    }
}

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;

    fn chain(&self, start: Cluster) -> Chain<Self> {
        Chain {
            vfat: self.clone(),
            next: Some(start),
        }
    }
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: BlockDeviceCached,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from_mbr_part0<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        let phy_sector_size = device.sector_size();
        let mbr = MasterBootRecord::from(&mut device)?;
        let part_data = mbr.partition_table[0];
        if part_data.partition_type != 0xB && part_data.partition_type != 0xC {
            // Paritition type is not FAT32
            return Err(Error::NotFound);
        }
        let start_phy_sector = part_data.relative_sector;
        let phy_sectors = part_data.total_sectors;
        let ebpb = BiosParameterBlock::from(&mut device, start_phy_sector as u64)?;
        let logical_sector_size = ebpb.bytes_per_sector;
        if (logical_sector_size as u64) < phy_sector_size {
            return Err(Error::Fat("logical sector size < physical sector size"));
        }
        let logical_sectors = ebpb.logical_sectors();
        let factor = logical_sector_size as u32 / phy_sector_size as u32;
        if logical_sectors > phy_sectors.saturating_mul(factor) {
            return Err(Error::Fat(
                "logical sectors exceeds physical sectors * factor",
            ));
        }
        let part = BlockDevicePartition::new(
            device,
            Partition {
                start: start_phy_sector as u64,
                num_sectors: logical_sectors as u64,
                sector_size: logical_sector_size as u64,
            },
        );
        let part_cached = BlockDeviceCached::new(part);
        let vfat = VFat {
            phantom: PhantomData::<HANDLE>,
            device: part_cached,
            bytes_per_sector: logical_sector_size,
            sectors_per_cluster: ebpb.sectors_per_cluster,
            sectors_per_fat: ebpb.sectors_per_fat(),
            fat_start_sector: ebpb.reserved_sectors as u64,
            data_start_sector: ebpb.reserved_sectors as u64
                + ebpb.fats as u64 * ebpb.sectors_per_fat() as u64,
            rootdir_cluster: Cluster::from(ebpb.rootdir_cluster),
        };
        Ok(HANDLE::new(vfat))
    }

    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        VFat::from_mbr_part0(device)
    }

    pub fn cluster_sector(&self, cluster: Cluster) -> u64 {
        self.data_start_sector + (cluster.raw() as u64 - 2) * self.sectors_per_cluster as u64
    }

    pub fn cluster_size(&self) -> u64 {
        self.device.sector_size() * self.sectors_per_cluster as u64
    }

    // Read from an offset of a cluster into a buffer.
    pub fn read_cluster(&mut self, cluster: Cluster, buf: &mut [u8]) -> io::Result<usize> {
        let sector = self.cluster_sector(cluster);
        read_n_sectors(
            &mut self.device,
            sector,
            self.sectors_per_cluster as usize,
            buf,
        )
    }

    // Read all of the clusters chained from a starting cluster into a vector.
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut cluster_data = vec![0; self.cluster_size() as usize];
        let mut next = start;
        let mut read_bytes = 0;
        // for cluster in self.chain(start) {
        //     let cluster = cluster?;
        //     read_bytes += self.read_cluster(cluster, &mut cluster_data)?;
        //     buf.extend_from_slice(&cluster_data);
        // }
        loop {
            read_bytes += self.read_cluster(next, &mut cluster_data)?;
            buf.extend_from_slice(&cluster_data);
            match self.fat_entry(next)?.status() {
                Status::Data(cluster) => next = cluster,
                Status::Eoc(_) => break,
                status => return ioerr!(InvalidData, "Invalid chain fat entry"),
            }
        }
        Ok(read_bytes)
    }

    // Return a reference to a `FatEntry` for a cluster where the reference points directly into a
    // cached sector.
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<FatEntry> {
        use core::mem::size_of;
        let fat_entries_per_sector = self.device.sector_size() as usize / size_of::<FatEntry>();
        let sector = self.fat_start_sector + cluster.raw() as u64 / (fat_entries_per_sector as u64);
        let offset = cluster.raw() as usize % (fat_entries_per_sector as usize);
        let offset_bytes = offset * size_of::<FatEntry>();
        let sector_data = self.device.get(sector)?;
        let mut bytes = [0; 4];
        bytes.copy_from_slice(&sector_data[offset_bytes..offset_bytes + 4]);
        Ok(FatEntry(u32::from_le_bytes(bytes)))
    }
}

const ROOTDIR_NAME: &'static str = "/";

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Entry = Entry<HANDLE>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        use crate::traits::Entry;
        use shim::path::Component;

        let mut dir = Dir {
            vfat: self.clone(),
            first_cluster: self.lock(|vfat| vfat.rootdir_cluster),
        };
        let mut components = path.as_ref().components().peekable();
        if let Some(Component::RootDir) = components.next() {
            ()
        } else {
            return ioerr!(NotFound, "directory not found");
        }
        while let Some(name) = components.next() {
            let entry = dir.find(name)?;
            if let None = components.peek() {
                return Ok(entry);
            } else {
                dir = match entry.into_dir() {
                    Some(dir) => dir,
                    None => return ioerr!(NotFound, "directory not found"),
                }
            }
        }
        Ok(Self::Entry {
            value: EntryValue::Dir(dir),
            _metadata: ROOTDIR_METADATA,
            _name: String::from(ROOTDIR_NAME),
        })
    }
}
