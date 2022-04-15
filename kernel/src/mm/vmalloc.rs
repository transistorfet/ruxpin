
use core::slice;

use alloc::vec::Vec;

use crate::mm::pages;
use crate::misc::align_up;
use crate::fs::types::File;
use crate::mm::segments::Segment;
use crate::mm::{MemoryType, MemoryPermissions};
use crate::arch::mmu::{self, TranslationTable};
use crate::arch::types::{VirtualAddress, PhysicalAddress};
use crate::errors::KernelError;


const MAX_SEGMENTS: usize = 6;


pub fn init_virtual_memory(start: PhysicalAddress, end: PhysicalAddress) {
    pages::init_pages_area(start, end);
}

pub struct VirtualAddressSpace {
    table: TranslationTable,
    segments: Vec<Segment>,
}

impl VirtualAddressSpace {
    pub fn new_user_space() -> Self {
        let pages = pages::get_page_area();
        let table = TranslationTable::new_user_table(pages);

        Self {
            table,
            segments: Vec::with_capacity(MAX_SEGMENTS),
        }
    }

    pub fn add_memory_segment(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) {
        let segment = Segment::new_memory(vaddr, vaddr.add(len));
        self.segments.push(segment);
        self.map_on_demand(permissions, vaddr, align_up(len, mmu::page_size()));
    }

    pub fn add_file_backed_segment(&mut self, permissions: MemoryPermissions, file: File, file_offset: usize, file_size: usize, vaddr: VirtualAddress, mem_offset: usize, mem_size: usize) {
        let segment = Segment::new_file_backed(file, file_offset, file_size, mem_offset, vaddr, vaddr.add(mem_size).add(mem_offset));
        self.segments.push(segment);
        self.map_on_demand(permissions, vaddr, align_up(mem_size + mem_offset, mmu::page_size()));
    }

    pub fn add_memory_segment_allocated(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) {
        let segment = Segment::new_memory(vaddr, vaddr.add(len));
        self.segments.push(segment);
        self.alloc_mapped(permissions, vaddr, align_up(len, mmu::page_size()));
    }

    pub fn clear_segments(&mut self) {
        self.unmap_range(VirtualAddress::from(0), 0xffff_ffff_ffff);

        self.segments.clear();
    }


    pub fn alloc_mapped(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) -> PhysicalAddress {
        let pages = pages::get_page_area();

        self.table.map_addr(MemoryType::Existing, permissions, vaddr, len, pages, &|pages, _, len| {
            if len == mmu::page_size() {
                Some(pages.alloc_page_zeroed())
            } else {
                None // Don't map granuales larger than a page
            }
        }).unwrap();

        self.table.translate_addr(vaddr).unwrap()
    }

    pub fn map_on_demand(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) {
        let pages = pages::get_page_area();
        self.table.map_addr(MemoryType::Unallocated, permissions, vaddr, len, pages, &|_, _, len| {
            if len == mmu::page_size() {
                Some(PhysicalAddress::from(0))
            } else {
                None
            }
        }).unwrap();
    }

    #[allow(dead_code)]
    pub fn map_existing(&mut self, permissions: MemoryPermissions, vaddr: VirtualAddress, paddr: PhysicalAddress, len: usize) {
        let pages = pages::get_page_area();
        self.table.map_addr(MemoryType::Existing, permissions, vaddr, len, pages, &|_, current_vaddr, _| {
            let voffset = usize::from(current_vaddr) - usize::from(vaddr);
            Some(paddr.add(voffset))
        }).unwrap();
    }

    pub fn unmap_range(&mut self, start: VirtualAddress, len: usize) {
        let pages = pages::get_page_area();
        self.table.unmap_addr(start, len, pages, &|pages, vaddr, paddr| {
            for segment in &self.segments {
                if segment.match_range(vaddr) {
                    // TODO this would normally call the segment operations to determine what to do
                    pages.free_page(paddr);
                }
            }
        }).unwrap();
    }

    pub fn translate_addr(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.table.translate_addr(vaddr)
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.table.get_ttbr()
    }

    pub(crate) fn alloc_page_at(&mut self, far: VirtualAddress) -> Result<(), KernelError> {
        for segment in &self.segments {
            if segment.match_range(far) {
                let pages = pages::get_page_area();

                // Allocate new page
                let page = pages.alloc_page_zeroed();
                let page_vaddr = far.align_down(mmu::page_size());
                self.table.update_mapping(page_vaddr, page, mmu::page_size()).unwrap();

                // Load data into the page if necessary
                let page_buffer = unsafe {
                    slice::from_raw_parts_mut(page.to_kernel_addr().as_mut(), mmu::page_size())
                };
                segment.ops.load_page_at(segment, page_vaddr, page_buffer).unwrap();

                return Ok(());
            }
        }

        Err(KernelError::NoSegmentFound)
    }
}

