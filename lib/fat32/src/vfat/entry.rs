use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

// TODO: Implement any useful helper methods on `Entry`.

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    // FIXME: Implement `traits::Entry` for `Entry`.
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    fn name(&self) -> &str {
        unimplemented!()
    }

    fn metadata(&self) -> &Self::Metadata {
        unimplemented!()
    }

    fn as_file(&self) -> Option<&<Self as traits::Entry>::File> {
        unimplemented!()
    }

    fn as_dir(&self) -> Option<&<Self as traits::Entry>::Dir> {
        unimplemented!()
    }

    fn into_file(self) -> Option<<Self as traits::Entry>::File> {
        unimplemented!()
    }

    fn into_dir(self) -> Option<<Self as traits::Entry>::Dir> {
        unimplemented!()
    }
}
