
use core::slice;

use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::mm::pages;
use crate::misc::align_up;
use crate::sync::Spinlock;
use crate::fs::types::File;
use crate::errors::KernelError;
use crate::mm::{MemoryType, MemoryPermissions};
use crate::mm::segments::{Segment, ArcSegment, SegmentType};
use crate::arch::mmu::{self, TranslationTable};
use crate::arch::types::{VirtualAddress, PhysicalAddress};


const MAX_SEGMENTS: usize = 6;


pub fn init_virtual_memory(start: PhysicalAddress, end: PhysicalAddress) {
    pages::init_pages_area(start, end);
}

pub type SharableVirtualAddressSpace = Arc<Spinlock<VirtualAddressSpace>>;

pub struct VirtualAddressSpace {
    table: TranslationTable,
    segments: Vec<ArcSegment>,
    data: Option<ArcSegment>,
}

impl VirtualAddressSpace {
    pub fn new_user_space() -> Self {
        let pages = pages::get_page_area();
        let table = TranslationTable::new_user_table(pages);

        Self {
            table,
            segments: Vec::with_capacity(MAX_SEGMENTS),
            data: None,
        }
    }

    pub fn new_sharable_user_space() -> SharableVirtualAddressSpace {
        Arc::new(Spinlock::new(Self::new_user_space()))
    }

    pub fn add_memory_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) {
        let segment = Arc::new(Spinlock::new(Segment::new_memory(permissions, vaddr, vaddr.add(len))));
        if stype != SegmentType::Stack && (self.data.is_none() || vaddr > self.data.as_mut().unwrap().lock().start) {
            self.data = Some(segment.clone());
        }
        self.segments.push(segment);
        self.map_on_demand(permissions, vaddr, align_up(len, mmu::page_size()));
    }

    pub fn add_file_backed_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, file: File, file_offset: usize, file_size: usize, vaddr: VirtualAddress, mem_offset: usize, mem_size: usize) {
        let segment = Arc::new(Spinlock::new(Segment::new_file_backed(file, file_offset, file_size, permissions, mem_offset, vaddr, vaddr.add(mem_size).add(mem_offset).align_up(mmu::page_size()))));
        if stype != SegmentType::Stack && (self.data.is_none() || vaddr > self.data.as_mut().unwrap().lock().start) {
            self.data = Some(segment.clone());
        }
        self.segments.push(segment);
        self.map_on_demand(permissions, vaddr, align_up(mem_size + mem_offset, mmu::page_size()));
    }

    pub fn add_memory_segment_allocated(&mut self, _stype: SegmentType, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) {
        let segment = Arc::new(Spinlock::new(Segment::new_memory(permissions, vaddr, vaddr.add(len))));
        self.segments.push(segment);
        self.alloc_mapped(permissions, vaddr, align_up(len, mmu::page_size()));
    }

    pub fn clear_segments(&mut self) {
        for i in 0..self.segments.len() {
            let start = self.segments[i].lock().start;
            let len = self.segments[i].lock().page_aligned_len();
            self.unmap_range(start, len);
        }
        self.segments.clear();
    }

    pub fn copy_segments(&mut self, parent: &mut Self) {
        for segment in parent.segments.iter() {
            //crate::printkln!("cloning segment {:x} to {:x}", usize::from(segment.start), usize::from(segment.end));
            self.segments.push(segment.clone());
            self.copy_segment_map(&mut parent.table, &*segment.lock());
        }
    }

    // TODO technically increment should be isize, and can be negative to shrink the size
    pub fn adjust_stack_break(&mut self, increment: usize) -> Result<VirtualAddress, KernelError> {
        let inc_aligned = align_up(increment, mmu::page_size());
        let segment = self.data.clone().unwrap();
        let mut locked_seg = segment.try_lock().unwrap();
        let previous_end = locked_seg.end;
        locked_seg.end = locked_seg.end.add(inc_aligned);
        self.map_on_demand(locked_seg.permissions, previous_end, inc_aligned);
        Ok(previous_end)
    }


    pub fn alloc_mapped(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) -> PhysicalAddress {
        let pages = pages::get_page_area();

        self.table.map_paged_range(MemoryType::Allocated, permissions, vaddr, len, pages).unwrap();

        self.table.translate_addr(vaddr).unwrap()
    }

    pub fn map_on_demand(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) {
        let pages = pages::get_page_area();
        self.table.map_paged_range(MemoryType::Unallocated, permissions, vaddr, len, pages).unwrap();
    }

    pub fn copy_segment_map(&mut self, parent_table: &mut TranslationTable, segment: &Segment) {
        let pages = pages::get_page_area();

        if segment.permissions == MemoryPermissions::ReadWrite {
            self.table.copy_pages_in_range(parent_table, segment.start, segment.page_aligned_len(), pages).unwrap();
        } else  {
            self.table.copy_paged_range(parent_table, segment.permissions, segment.start, segment.page_aligned_len(), pages).unwrap();
        }
    }

    pub fn unmap_range(&mut self, start: VirtualAddress, len: usize) {
        let pages = pages::get_page_area();

        self.table.unmap_range(start, len, pages).unwrap();
    }

    pub fn translate_addr(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.table.translate_addr(vaddr)
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.table.get_ttbr()
    }

    pub(crate) fn alloc_page_at(&mut self, far: VirtualAddress) -> Result<(), KernelError> {
        for segment in &self.segments {
            let locked_seg = segment.try_lock().unwrap();
            if locked_seg.match_range(far) {
                let pages = pages::get_page_area();

                // Allocate new page
                let page = pages.alloc_page_zeroed();
                let page_vaddr = far.align_down(mmu::page_size());
                self.table.update_addr(page_vaddr, page, mmu::page_size()).unwrap();

                // Load data into the page if necessary
                let page_buffer = mmu::get_page_slice(page);
                locked_seg.load_page_at(&*locked_seg, page_vaddr, page_buffer).unwrap();

                return Ok(());
            }
        }

        Err(KernelError::NoSegmentFound)
    }
}

