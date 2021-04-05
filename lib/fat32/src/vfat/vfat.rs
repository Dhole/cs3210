use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, BlockDeviceCached, BlockDevicePartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    // device: CachedPartition,
    device: Box<dyn BlockDevice>,
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
        Ok(HANDLE::new(VFat {
            phantom: PhantomData::<HANDLE>,
            device: Box::new(part_cached),
            bytes_per_sector: logical_sector_size,
            sectors_per_cluster: ebpb.sectors_per_cluster,
            sectors_per_fat: ebpb.sectors_per_fat(),
            fat_start_sector: ebpb.reserved_sectors as u64,
            data_start_sector: ebpb.reserved_sectors as u64
                + ebpb.fats as u64 * ebpb.sectors_per_fat() as u64,
            rootdir_cluster: Cluster::from(ebpb.rootdir_cluster),
        }))
    }

    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        unimplemented!();
    }

    // TODO: The following methods may be useful here:
    //
    //  * A method to read from an offset of a cluster into a buffer.
    //
    //    fn read_cluster(
    //        &mut self,
    //        cluster: Cluster,
    //        offset: usize,
    //        buf: &mut [u8]
    //    ) -> io::Result<usize>;
    //
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    //    fn read_chain(
    //        &mut self,
    //        start: Cluster,
    //        buf: &mut Vec<u8>
    //    ) -> io::Result<usize>;
    //
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    //    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry>;
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = crate::traits::Dummy;
    type Dir = crate::traits::Dummy;
    type Entry = crate::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }
}
