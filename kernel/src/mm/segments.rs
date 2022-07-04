
use alloc::sync::Arc;
use alloc::boxed::Box;

use crate::arch::mmu::{self, TranslationTable};
use crate::arch::{PhysicalAddress, VirtualAddress};
use crate::errors::KernelError;
use crate::misc::align_up;
use crate::sync::Spinlock;

use super::pages;
use super::MemoryPermissions;
use super::pagecache::PageCacheEntry;


#[derive(Copy, Clone, PartialEq)]
pub enum SegmentType {
    Text,
    Data,
    Stack,
}

pub trait SegmentOperations: Sync + Send {
    fn load_page_at(&self, segment: &Segment, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError>;
}

pub struct Segment {
    pub(super) permissions: MemoryPermissions,
    pub(super) start: VirtualAddress,
    pub(super) end: VirtualAddress,
    ops: Box<dyn SegmentOperations>,
}

pub type ArcSegment = Arc<Spinlock<Segment>>;

impl Segment {
    pub fn new(permissions: MemoryPermissions, start: VirtualAddress, end: VirtualAddress, ops: Box<dyn SegmentOperations>) -> Self {
        Self {
            permissions,
            start,
            end,
            ops,
        }
    }

    pub fn new_memory(permissions: MemoryPermissions, start: VirtualAddress, end: VirtualAddress) -> Result<Self, KernelError> {
        let ops = Box::new(MemorySegment::new()?);
        Ok(Self::new(permissions, start, end, ops))
    }

    pub fn new_file_backed(cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize, permissions: MemoryPermissions, mem_offset: usize, start: VirtualAddress, end: VirtualAddress) -> Result<Self, KernelError> {
        let ops = Box::new(FileBackedSegment::new(cache, file_offset, file_size, mem_offset)?);
        Ok(Self::new(permissions, start, end, ops))
    }

    pub fn page_aligned_len(&self) -> usize {
        align_up(usize::from(self.end) - usize::from(self.start), mmu::page_size())
    }

    pub fn match_range(&self, addr: VirtualAddress) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn load_page_at(&self, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.ops.load_page_at(&self, table, vaddr)
    }
}



#[derive(Clone)]
pub struct MemorySegment {}

impl MemorySegment {
    pub fn new() -> Result<Self, KernelError> {
        Ok(Self {})
    }
}

impl SegmentOperations for MemorySegment {
    fn load_page_at(&self, _segment: &Segment, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        let pages = pages::get_page_pool();
        let page = pages.alloc_page_zeroed();
        table.update_page_addr(vaddr, page).unwrap();
        Ok(page)
    }
}


#[derive(Clone)]
pub struct FileBackedSegment {
    cache: Arc<PageCacheEntry>,
    file_offset: usize,
    file_size: usize,
    mem_offset: usize,
}

impl FileBackedSegment {
    pub fn new(cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize, mem_offset: usize) -> Result<Self, KernelError> {
        Ok(Self {
            cache,
            file_offset,
            file_size,
            mem_offset,
        })
    }
}

impl SegmentOperations for FileBackedSegment {
    fn load_page_at(&self, segment: &Segment, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        let offset = usize::from(vaddr) - usize::from(segment.start) - (self.mem_offset - self.file_offset);
        // TODO if the request is beyond the end of the file, then you could allocate a normal page instead of using a cached one, and save the copy on write

        let page = self.cache.lookup(offset)?;
        table.update_page_addr(vaddr, page).unwrap();
        if segment.permissions == MemoryPermissions::ReadWrite {
            table.set_page_copy_on_write(vaddr).unwrap();
        }
        Ok(page)
    }
}

