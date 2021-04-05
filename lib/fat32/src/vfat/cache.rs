use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp;
use core::fmt;
use hashbrown::{hash_map::Entry, HashMap};
use shim::io;
use shim::ioerr;

use crate::traits::BlockDevice;

#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}

#[derive(Debug)]
pub struct Partition {
    /// The physical sector where the partition begins.
    pub start: u64,
    /// Number of sectors
    pub num_sectors: u64,
    /// The size, in bytes, of a logical sector in the partition.
    pub sector_size: u64,
}

pub struct BlockDevicePartition {
    device: Box<dyn BlockDevice>,
    partition: Partition,
}

impl fmt::Debug for BlockDevicePartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BlockDevice")
            .field("device", &self.device)
            .field("partition", &self.partition)
            .finish()
    }
}

impl BlockDevicePartition {
    pub fn new<T>(device: T, partition: Partition) -> Self
    where
        T: BlockDevice + 'static,
    {
        assert!(partition.sector_size >= device.sector_size());
        Self {
            device: Box::new(device),
            partition: partition,
        }
    }

    /// Returns the number of physical sectors that corresponds to
    /// one logical sector.
    fn factor(&self) -> u64 {
        self.partition.sector_size / self.device.sector_size()
    }

    /// Maps a user's request for a sector `virt` to the physical sector.
    /// Returns `None` if the virtual sector number is out of range.
    fn virtual_to_physical(&self, virt: u64) -> Option<u64> {
        if virt >= self.partition.num_sectors {
            return None;
        }

        let physical_offset = virt * self.factor();
        let physical_sector = self.partition.start + physical_offset;

        Some(physical_sector)
    }
}

impl BlockDevice for BlockDevicePartition {
    fn sector_size(&self) -> u64 {
        self.partition.sector_size
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        let phy_sector = match self.virtual_to_physical(sector) {
            Some(s) => s,
            None => return ioerr!(InvalidInput, "virtual sector out of range"),
        };
        let phy_sector_size = self.device.sector_size() as usize;
        let buf_len = buf.len();
        let mut read_bytes = 0;
        for i in 0..self.factor() as usize {
            let n = self.device.read_sector(
                phy_sector + i as u64,
                &mut buf[i * phy_sector_size..cmp::min(buf_len, (i + 1) * phy_sector_size)],
            )?;
            read_bytes += n;
            if n < phy_sector_size {
                break;
            }
        }
        Ok(read_bytes)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        let phy_sector = match self.virtual_to_physical(sector) {
            Some(s) => s,
            None => return ioerr!(InvalidInput, "virtual sector out of range"),
        };
        let phy_sector_size = self.device.sector_size() as usize;
        let buf_len = buf.len();
        let mut write_bytes = 0;
        for i in 0..self.factor() as usize {
            let n = self.device.write_sector(
                phy_sector + i as u64,
                &buf[i * phy_sector_size..cmp::min(buf_len, (i + 1) * phy_sector_size)],
            )?;
            write_bytes += n;
            if n < phy_sector_size {
                break;
            }
        }
        Ok(write_bytes)
    }
}

pub struct BlockDeviceCached {
    device: Box<dyn BlockDevice>,
    cache: HashMap<u64, CacheEntry>,
}

impl fmt::Debug for BlockDeviceCached {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BlockDeviceCached")
            .field("device", &self.device)
            .field("cache", &self.cache)
            .finish()
    }
}

impl BlockDeviceCached {
    pub fn new<T>(device: T) -> Self
    where
        T: BlockDevice + 'static,
    {
        Self {
            device: Box::new(device),
            cache: HashMap::new(),
        }
    }

    /// Returns a mutable reference to the cached sector `sector`. If the sector
    /// is not already cached, the sector is first read from the disk.
    ///
    /// The sector is marked dirty as a result of calling this method as it is
    /// presumed that the sector will be written to. If this is not intended,
    /// use `get()` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get_mut(&mut self, sector: u64) -> io::Result<&mut [u8]> {
        let mut cache_entry = match self.cache.entry(sector) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mut sector_data = vec![0; self.device.sector_size() as usize];
                self.device.read_sector(sector, &mut sector_data)?;
                entry.insert(CacheEntry {
                    data: sector_data,
                    dirty: false,
                })
            }
        };
        cache_entry.dirty = true;
        Ok(&mut cache_entry.data)
    }

    /// Returns a reference to the cached sector `sector`. If the sector is not
    /// already cached, the sector is first read from the disk.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get(&mut self, sector: u64) -> io::Result<&[u8]> {
        let cache_entry = match self.cache.entry(sector) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mut sector_data = vec![0; self.device.sector_size() as usize];
                self.device.read_sector(sector, &mut sector_data)?;
                entry.insert(CacheEntry {
                    data: sector_data,
                    dirty: false,
                })
            }
        };
        Ok(&cache_entry.data)
    }
}

impl BlockDevice for BlockDeviceCached {
    fn sector_size(&self) -> u64 {
        self.device.sector_size()
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        let read_bytes = cmp::min(self.sector_size() as usize, buf.len());
        let sector_data = self.get(sector)?;
        buf[..read_bytes].copy_from_slice(&sector_data[..read_bytes]);
        Ok(read_bytes)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        let write_bytes = cmp::min(self.sector_size() as usize, buf.len());
        let sector_data = self.get_mut(sector)?;
        sector_data[..write_bytes].copy_from_slice(&buf[..write_bytes]);
        Ok(write_bytes)
    }
}

#[derive(Debug)]
pub struct CachedPartition {
    device: Box<dyn BlockDevice>,
}

impl CachedPartition {
    /// Creates a new `CachedPartition` that transparently caches sectors from
    /// `device` and maps physical sectors to logical sectors inside of
    /// `partition`. All reads and writes from `CacheDevice` are performed on
    /// in-memory caches.
    ///
    /// The `partition` parameter determines the size of a logical sector and
    /// where logical sectors begin. An access to a sector `0` will be
    /// translated to physical sector `partition.start`. Virtual sectors of
    /// sector number `[0, num_sectors)` are accessible.
    ///
    /// `partition.sector_size` must be an integer multiple of
    /// `device.sector_size()`.
    ///
    /// # Panics
    ///
    /// Panics if the partition's sector size is < the device's sector size.
    pub fn new<T>(device: T, partition: Partition) -> CachedPartition
    where
        T: BlockDevice + 'static,
    {
        let device = BlockDevicePartition::new(device, partition);
        let device = BlockDeviceCached::new(device);
        CachedPartition {
            device: Box::new(device),
        }
    }
}

// FIXME: Implement `BlockDevice` for `CacheDevice`. The `read_sector` and
// `write_sector` methods should only read/write from/to cached sectors.
impl BlockDevice for CachedPartition {
    fn sector_size(&self) -> u64 {
        self.device.sector_size()
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.device.read_sector(sector, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        self.device.write_sector(sector, &buf)
    }
}
