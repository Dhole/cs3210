use alloc::string::String;

use shim::io::{self, SeekFrom};
use shim::{ioerr, newioerr};

use crate::traits;
// use crate::util::print_hex;
use crate::vfat::{Chain, Cluster, Metadata, VFatHandle};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    cluster_size: u64,
    pub first_cluster: Cluster,
    pub current_cluster: Cluster,
    // pub chain: Chain<HANDLE>,
    pub size: u32,
    pub pos: u64,
}

impl<HANDLE: VFatHandle> File<HANDLE> {
    pub fn new(vfat: HANDLE, first_cluster: Cluster, size: u32) -> File<HANDLE> {
        // let chain = vfat.chain(first_cluster);
        let cluster_size = vfat.lock(|vfat| vfat.cluster_size());
        File {
            vfat,
            cluster_size,
            first_cluster,
            current_cluster: first_cluster,
            // chain,
            size,
            pos: 0,
        }
    }
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    fn sync(&mut self) -> io::Result<()> {
        // TODO
        Ok(())
    }

    fn size(&self) -> u64 {
        self.size as u64
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, seek: SeekFrom) -> io::Result<u64> {
        use traits::File;

        let pos = match seek {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(i) => (self.size() as i64).saturating_add(i),
            SeekFrom::Current(i) => (self.pos as i64).saturating_add(i),
        };
        if pos < 0 || pos > (self.size() as i64) {
            return ioerr!(InvalidInput, "seek outside file");
        };
        self.pos = pos as u64;

        let cluster_index = self.pos / self.cluster_size;
        let cluster_offset = self.pos % self.cluster_size;
        let mut chain = self.vfat.chain(self.first_cluster);
        self.current_cluster = chain
            .skip(cluster_index as usize)
            .nth(0)
            .ok_or(newioerr!(InvalidInput, "position past the end of file"))??;

        Ok(self.pos)
    }
}

impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use traits::File;

        if self.pos == self.size as u64 {
            return Ok(0);
        } else if self.pos >= self.size as u64 {
            return ioerr!(InvalidInput, "read past the end of file");
        }
        let cluster_index = self.pos / self.cluster_size;
        let cluster_offset = self.pos % self.cluster_size;

        let cluster_data_len = if cluster_index == self.size as u64 / self.cluster_size {
            self.size as u64 % self.cluster_size
        } else {
            self.cluster_size
        };
        let mut cluster_data = vec![0; cluster_data_len as usize];
        self.vfat.lock(|vfat| -> io::Result<()> {
            vfat.read_cluster(self.current_cluster, &mut cluster_data)?;
            Ok(())
        });
        let len = core::cmp::min(buf.len() as u64, cluster_data_len - cluster_offset) as usize;
        buf[..len]
            .copy_from_slice(&cluster_data[cluster_offset as usize..cluster_offset as usize + len]);
        self.pos += len as u64;

        // If we just read until the end of the cluster, and it's not the last cluster, preload the
        // next cluster index
        if self.pos != self.size as u64
            && cluster_offset as usize + len == cluster_data_len as usize
        {
            let mut chain = self.vfat.chain(self.current_cluster);
            self.current_cluster = chain
                .nth(1)
                .ok_or(newioerr!(InvalidInput, "position past the end of file"))??;
        }
        Ok(len)
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO
        unimplemented!()
    }
    fn flush(&mut self) -> io::Result<()> {
        // TODO
        unimplemented!()
    }
}
