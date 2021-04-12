use alloc::string::String;

use shim::io::{self, SeekFrom};
use shim::ioerr;

use crate::traits;
use crate::util::print_hex;
use crate::vfat::{Cluster, Metadata, VFatHandle};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub first_cluster: Cluster,
    pub size: u32,
    pub pos: u64,
    // FIXME: Fill me in.
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
        Ok(self.pos)
    }
}

impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use traits::File;

        if self.pos >= self.size as u64 {
            return Ok(0);
        }
        // TODO: Replace by an efficient implementation
        let mut file_buf = Vec::new();
        self.vfat.lock(|vfat| -> io::Result<()> {
            vfat.read_chain(self.first_cluster, &mut file_buf)?;
            Ok(())
        });
        // if self.pos == 0 {
        //     println!("DBG size: {}", self.size());
        //     print_hex(&file_buf[..self.size() as usize]);
        // }
        // if file_buf.len() < self.size() as usize {
        //     panic!(
        //         "file_buf.len(): {} < self.size(): {}",
        //         file_buf.len(),
        //         self.size()
        //     );
        // }
        let len = core::cmp::min(buf.len() as u64, self.size() - self.pos) as usize;
        // if self.pos as usize >= file_buf.len() {
        //     println!("DBG file read past");
        //     for b in buf[..len].iter_mut() {
        //         *b = 0x00;
        //     }
        //     self.pos += len as u64;
        //     return Ok(len);
        // }
        // let len = core::cmp::min(len, file_buf.len() - self.pos as usize);
        buf[..len].copy_from_slice(&file_buf[self.pos as usize..self.pos as usize + len]);
        self.pos += len as u64;
        Ok(len)
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO
        Ok(0)
    }
    fn flush(&mut self) -> io::Result<()> {
        // TODO
        Ok(())
    }
}
