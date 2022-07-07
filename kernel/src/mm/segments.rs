
use alloc::sync::Arc;
use alloc::boxed::Box;

use crate::arch::mmu::{self, TranslationTable};
use crate::arch::{PhysicalAddress, VirtualAddress};
use crate::errors::KernelError;
use crate::misc::align_up;

use super::pages;
use super::{MemoryType, MemoryPermissions};
use super::pagecache::PageCacheEntry;


#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SegmentType {
    Text,
    Data,
    Stack,
}

pub trait SegmentOperations: Sync + Send {
    fn copy(&self) -> Box<dyn SegmentOperations>;
    fn load_page_at(&self, segment: &Segment, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError>;
}

pub struct Segment {
    pub(super) stype: SegmentType,
    pub(super) permissions: MemoryPermissions,
    pub(super) start: VirtualAddress,
    pub(super) end: VirtualAddress,
    ops: Box<dyn SegmentOperations>,
}

impl Segment {
    pub fn new(stype: SegmentType, permissions: MemoryPermissions, start: VirtualAddress, end: VirtualAddress, ops: Box<dyn SegmentOperations>) -> Self {
        Self {
            stype,
            permissions,
            start,
            end,
            ops,
        }
    }

    pub fn new_memory(table: &mut TranslationTable, stype: SegmentType, permissions: MemoryPermissions, start: VirtualAddress, end: VirtualAddress) -> Result<Self, KernelError> {
        let ops = Box::new(MemorySegment::new()?);
        let segment = Self::new(stype, permissions, start, end, ops);
        let pages = pages::get_page_pool();
        table.map_paged_range(MemoryType::Unallocated, permissions, segment.start, segment.page_aligned_len(), pages)?;
        Ok(segment)
    }

    pub fn new_file_backed(table: &mut TranslationTable, stype: SegmentType, permissions: MemoryPermissions, mem_offset: usize, start: VirtualAddress, end: VirtualAddress, cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize) -> Result<Self, KernelError> {
        let ops = Box::new(FileBackedSegment::new(cache, file_offset, file_size, mem_offset)?);
        let segment = Self::new(stype, permissions, start, end, ops);
        let pages = pages::get_page_pool();
        table.map_paged_range(MemoryType::Unallocated, permissions, segment.start, segment.page_aligned_len(), pages)?;
        Ok(segment)
    }

    pub fn page_aligned_len(&self) -> usize {
        align_up(usize::from(self.end) - usize::from(self.start), mmu::page_size())
    }

    pub fn match_range(&self, addr: VirtualAddress) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn unmap(&mut self, table: &mut TranslationTable) -> Result<(), KernelError> {
        let pages = pages::get_page_pool();
        table.unmap_range(self.start, self.page_aligned_len(), pages)
    }

    pub fn load_page_at(&self, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.ops.load_page_at(&self, table, vaddr)
    }

    pub fn copy(&self, table: &mut TranslationTable, parent_table: &mut TranslationTable) -> Result<Self, KernelError> {
        let pages = pages::get_page_pool();

        if self.permissions == MemoryPermissions::ReadWrite {
            table.remap_range_copy_on_write(parent_table, self.start, self.page_aligned_len(), pages)?;
        } else  {
            table.duplicate_paged_range(parent_table, self.permissions, self.start, self.page_aligned_len(), pages)?;
        }
        Ok(Self::new(self.stype, self.permissions, self.start, self.end, self.ops.copy()))
    }

    pub fn resize(&mut self, table: &mut TranslationTable, diff: isize) -> Result<(), KernelError> {
        let pages = pages::get_page_pool();

        if diff >= 0 {
            let aligned_diff = align_up(diff as usize, mmu::page_size());
            table.map_paged_range(MemoryType::Unallocated, self.permissions, self.end, aligned_diff, pages)?;
            self.end = self.end.add(aligned_diff);
        } else {
            let aligned_diff = align_up((-1 * diff) as usize, mmu::page_size());
            table.unmap_range(self.end.sub(aligned_diff), aligned_diff, pages)?;
            self.end = self.end.sub(aligned_diff);
        }
        Ok(())
    }

    pub fn resize_stack(&mut self, table: &mut TranslationTable, diff: isize) -> Result<(), KernelError> {
        let pages = pages::get_page_pool();

        if diff >= 0 {
            let aligned_diff = align_up(diff as usize, mmu::page_size());
            table.map_paged_range(MemoryType::Unallocated, self.permissions, self.start.sub(aligned_diff), aligned_diff, pages)?;
            self.start = self.start.sub(aligned_diff);
        } else {
            let aligned_diff = align_up((-1 * diff) as usize, mmu::page_size());
            table.unmap_range(self.start, aligned_diff, pages)?;
            self.start = self.start.add(aligned_diff);
        }
        Ok(())
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
    fn copy(&self) -> Box<dyn SegmentOperations> {
        Box::new(self.clone())
    }

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
    file_limit: usize,
    mem_offset: usize,
}

impl FileBackedSegment {
    pub fn new(cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize, mem_offset: usize) -> Result<Self, KernelError> {
        Ok(Self {
            cache,
            file_offset,
            file_limit: align_up(file_offset + file_size, mmu::page_size()),
            mem_offset,
        })
    }
}

impl SegmentOperations for FileBackedSegment {
    fn copy(&self) -> Box<dyn SegmentOperations> {
        Box::new(self.clone())
    }

    fn load_page_at(&self, segment: &Segment, table: &mut TranslationTable, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        let offset = usize::from(vaddr) - usize::from(segment.start) - (self.mem_offset - self.file_offset);

        if offset <= self.file_limit {
            let page = self.cache.lookup(offset)?;
            table.update_page_addr(vaddr, page).unwrap();
            if segment.permissions == MemoryPermissions::ReadWrite {
                table.set_page_copy_on_write(vaddr).unwrap();
            }
            Ok(page)
        } else {
            let pages = pages::get_page_pool();
            let page = pages.alloc_page_zeroed();
            table.update_page_addr(vaddr, page).unwrap();
            Ok(page)
        }
    }
}

