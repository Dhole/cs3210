use alloc::string::String;
use core::fmt;
use shim::const_assert_size;

use crate::traits::BlockDevice;
use crate::vfat::Error;

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct BiosParameterBlock {
    jmp_short_xx_nop: [u8; 3],
    _oem_id: [u8; 8],
    pub bytes_per_sector: u16,   // bytes per logical sector
    pub sectors_per_cluster: u8, // logical sectors per cluster
    pub reserved_sectors: u16,   // reserved logical sectors
    pub fats: u8,
    pub max_num_dir: u16,
    pub logical_sectors_16: u16,
    pub fat_id: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub heads: u16,
    pub hidden_sectors: u32,
    pub logical_sectors_32: u32, // total logical sectors
    pub sectors_per_fat_32: u32, // logical sectors per fat
    pub flags: u16,
    pub fat_version: [u8; 2],
    pub rootdir_cluster: u32,
    pub fsinfo_sector: u16,
    pub boot_sector_backup_sector: u16,
    pub reserved: [u8; 12],
    pub drive_num: u8,
    pub flags_winnt: u8,
    pub signature: u8,
    pub volume_id: u32,
    _volume_label: [u8; 11],
    _system_id: [u8; 8],
    pub boot_code: [u8; 420],
    pub boot_part_signature: [u8; 2],
}

const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut sector_data = vec![0; device.sector_size() as usize];
        device.read_sector(sector, &mut sector_data)?;
        let ebpb = unsafe { *{ sector_data.as_ptr() as *const BiosParameterBlock } };
        if ebpb.signature != 0x28 && ebpb.signature != 0x29 {
            return Err(Error::BadSignature);
        }
        Ok(ebpb)
    }

    pub fn logical_sectors(&self) -> u32 {
        if self.logical_sectors_16 == 0 {
            self.logical_sectors_32
        } else {
            self.logical_sectors_16 as u32
        }
    }

    pub fn sectors_per_fat(&self) -> u32 {
        if self.sectors_per_fat_16 == 0 {
            self.sectors_per_fat_32
        } else {
            self.sectors_per_fat_16 as u32
        }
    }

    pub fn oem_id(&self) -> alloc::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self._oem_id)
    }

    pub fn volume_label(&self) -> alloc::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self._volume_label)
    }

    pub fn system_id(&self) -> alloc::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self._system_id)
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterBlock")
            .field("oem_id", &self.oem_id())
            .field("bytes_per_sector", &self.bytes_per_sector)
            .field("sectors_per_cluster", &self.sectors_per_cluster)
            .field("reserved_sectors", &self.reserved_sectors)
            .field("fats", &self.fats)
            .field("max_num_dir", &self.max_num_dir)
            .field("logical_sectors", &self.logical_sectors())
            .field("fat_id", &self.fat_id)
            .field("sectors_per_fat", &self.sectors_per_fat())
            .field("sectors_per_track", &self.sectors_per_track)
            .field("heads", &self.heads)
            .field("hidden_sectors", &self.hidden_sectors)
            .field("flags", &self.flags)
            .field("fat_version", &self.fat_version)
            .field("rootdir_cluster", &self.rootdir_cluster)
            .field("fsinfo_sector", &self.fsinfo_sector)
            .field("boot_sector_backup_sector", &self.boot_sector_backup_sector)
            .field("drive_num", &self.drive_num)
            .field("flags_winnt", &self.flags_winnt)
            .field("signature", &self.signature)
            .field("volume_id", &self.volume_id)
            .field("volume_label", &self.volume_label())
            .field("system_id", &self.system_id())
            .field("boot_part_signature", &self.boot_part_signature)
            .finish()
    }
}
