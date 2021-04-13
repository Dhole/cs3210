use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Debug)]
pub enum EntryValue<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

// You can change this definition if you want
#[derive(Debug)]
pub struct Entry<HANDLE: VFatHandle> {
    pub value: EntryValue<HANDLE>,
    pub _metadata: Metadata,
    pub _name: String,
}

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    fn name(&self) -> &str {
        &self._name
    }

    fn metadata(&self) -> &Self::Metadata {
        &self._metadata
    }

    fn as_file(&self) -> Option<&<Self as traits::Entry>::File> {
        match self.value {
            EntryValue::File(ref file) => Some(file),
            EntryValue::Dir(_) => None,
        }
    }

    fn as_dir(&self) -> Option<&<Self as traits::Entry>::Dir> {
        match self.value {
            EntryValue::File(_) => None,
            EntryValue::Dir(ref dir) => Some(dir),
        }
    }

    fn into_file(self) -> Option<<Self as traits::Entry>::File> {
        match self.value {
            EntryValue::File(file) => Some(file),
            EntryValue::Dir(_) => None,
        }
    }

    fn into_dir(self) -> Option<<Self as traits::Entry>::Dir> {
        match self.value {
            EntryValue::File(_) => None,
            EntryValue::Dir(dir) => Some(dir),
        }
    }
}
