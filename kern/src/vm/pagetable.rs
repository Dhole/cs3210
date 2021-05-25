use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::slice::Iter;

use alloc::boxed::Box;
use alloc::fmt;
use core::alloc::{GlobalAlloc, Layout};

use crate::allocator;
use crate::allocator::util::align_up;
use crate::console::{kprint, kprintln};
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::ALLOCATOR;

use pi::common::{IO_BASE, IO_BASE_END};

use aarch64::vmsa::*;
use shim::const_assert_size;

#[repr(C)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L2PageTable {
    pub entries: [RawL2Entry; 8192],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
        Self {
            entries: [RawL2Entry::new(0); 8192],
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(&self.entries as *const RawL2Entry as u64)
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
        Self(RawL3Entry::new(0))
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
        self.0.get_masked(RawL3Entry::VALID) != 0
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
        if self.is_valid() {
            Some(PhysicalAddr::from(self.0.get_masked(RawL3Entry::ADDR)))
        } else {
            None
        }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L3PageTable {
    pub entries: [L3Entry; 8192],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
        Self {
            entries: [L3Entry::new(); 8192],
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(&self.entries as *const L3Entry as u64)
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [L3PageTable; 2],
}

impl PageTable {
    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {
        let mut pt = Box::new(PageTable {
            l2: L2PageTable::new(),
            l3: [L3PageTable::new(), L3PageTable::new()],
        });
        for i in 0..2 {
            let addr = pt.l3[i].as_ptr();
            kprintln!("l3[{}] addr = 0x{:x}", i, addr.as_u64());
            let mut entry = &mut pt.l2.entries[i];
            entry.set_masked(addr.as_u64(), RawL2Entry::ADDR);
            entry.set_bit(RawL2Entry::AF);
            entry.set_value(EntrySh::ISh, RawL2Entry::SH);
            entry.set_value(perm, RawL2Entry::AP);
            entry.set_value(EntryAttr::Mem, RawL2Entry::ATTR);
            entry.set_value(EntryType::Table, RawL2Entry::TYPE);
            entry.set_bit(RawL2Entry::VALID);
        }
        pt
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// Since we are only supporting 1GB virtual memory in this system, L2index
    /// should be smaller than 2.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
        let va = VirtualAddrEntry::new(va.as_u64());
        let l2_index = va.get_value(VirtualAddrEntry::L2INDEX);
        let l3_index = va.get_value(VirtualAddrEntry::L3INDEX);
        let pa = va.get_value(VirtualAddrEntry::PA);
        if l2_index > 2 {
            panic!("l2_index > 2: {}", l2_index);
        }
        if pa != 0 {
            panic!("pa != 0: 0x{:x}", pa);
        }
        (l2_index as usize, l3_index as usize)
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        let (l2_index, l3_index) = Self::locate(va);
        let entry = self.l3[l2_index].entries[l3_index];
        entry.is_valid()
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.is_valid(va)
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        let (l2_index, l3_index) = Self::locate(va);
        self.l3[l2_index].entries[l3_index] = L3Entry(entry);
        self
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        self.l2.as_ptr()
    }
}

use core::slice;

// FIXME: Implement `IntoIterator` for `&PageTable`.
impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a L3Entry;
    type IntoIter = Chain<slice::Iter<'a, L3Entry>, slice::Iter<'a, L3Entry>>;

    // impl PageTable {
    fn into_iter(self) -> Self::IntoIter {
        self.l3[0].entries.iter().chain(self.l3[1].entries.iter())
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
        let mut pt = PageTable::new(EntryPerm::KERN_RW);
        let (_, end_addr) = allocator::memory_map().unwrap();
        for addr in (0..end_addr).step_by(PAGE_SIZE) {
            let va = VirtualAddr::from(addr);
            let mut entry = RawL3Entry::new(0);
            entry.set_masked(addr as u64, RawL3Entry::ADDR);
            entry.set_bit(RawL3Entry::AF);
            entry.set_value(EntrySh::ISh, RawL3Entry::SH);
            entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
            entry.set_value(EntryType::Table, RawL3Entry::TYPE);
            entry.set_bit(RawL3Entry::VALID);
            pt.set_entry(va, entry);
        }
        for addr in (IO_BASE..IO_BASE_END).step_by(PAGE_SIZE) {
            let va = VirtualAddr::from(addr);
            let mut entry = RawL3Entry::new(0);
            entry.set_masked(addr as u64, RawL3Entry::ADDR);
            entry.set_bit(RawL3Entry::AF);
            entry.set_value(EntrySh::OSh, RawL3Entry::SH);
            entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            entry.set_value(EntryAttr::Dev, RawL3Entry::ATTR);
            entry.set_value(EntryType::Table, RawL3Entry::TYPE);
            entry.set_bit(RawL3Entry::VALID);
            pt.set_entry(va, entry);
        }
        kprintln!("l2.entries[..2]");
        kprintln!("  {:?}", pt.l2.entries[0]);
        kprintln!("  {:?}", pt.l2.entries[1]);
        kprintln!("l3[0].entries[..10]");
        for entry in &pt.l3[0].entries[..10] {
            kprintln!("  {:?}", entry.0);
        }
        Self(pt)
    }
}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    pub fn new() -> UserPageTable {
        Self(PageTable::new(EntryPerm::USER_RW))
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    pub fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        if va.as_usize() < USER_IMG_BASE {
            panic!("va < USER_IMG_BASE: 0x{:x}", va.as_u64());
        }
        let va_offset = va - VirtualAddr::from(USER_IMG_BASE);
        if self.0.is_valid(va_offset) {
            panic!("va already allocated: 0x{:x}", va.as_u64());
        }
        let addr = unsafe { ALLOCATOR.alloc(Page::layout()) as u64 };
        kprintln!("allocated at 0x{:x}", addr);
        if addr == 0 {
            panic!("allocation failed");
        }
        let mut entry = RawL3Entry::new(0);
        entry.set_masked(addr as u64, RawL3Entry::ADDR);
        entry.set_bit(RawL3Entry::AF);
        entry.set_value(EntrySh::ISh, RawL3Entry::SH);
        entry.set_value(EntryPerm::USER_RW, RawL3Entry::AP);
        entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
        entry.set_value(EntryType::Table, RawL3Entry::TYPE);
        entry.set_bit(RawL3Entry::VALID);
        self.0.set_entry(va_offset, entry);
        kprintln!(
            "UserPageTable.alloc at 0x{:x} {:?}",
            va_offset.as_u64(),
            entry
        );
        unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, PAGE_SIZE) }
    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UserPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for UserPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// FIXME: Implement `Drop` for `UserPageTable`.
impl Drop for UserPageTable {
    fn drop(&mut self) {
        for entry in self.0.into_iter() {
            if entry.is_valid() {
                let addr = entry.get_page_addr().unwrap();
                unsafe {
                    ALLOCATOR.dealloc(addr.as_u64() as *mut u8, Page::layout());
                }
            }
        }
    }
}

// FIXME: Implement `fmt::Debug` as you need.
impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "UserPageTable {{")?;
        for entry in self.0.into_iter() {
            if entry.is_valid() {
                writeln!(f, "  0x{:08x}", entry.get_page_addr().unwrap().as_u64())?;
            }
        }
        write!(f, "}}")?;
        Ok(())
    }
}
