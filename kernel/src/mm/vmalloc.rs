
use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::trace;
use crate::mm::pages;
use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::arch::mmu::{self, TranslationTable};
use crate::arch::{VirtualAddress, PhysicalAddress};

use super::MemoryPermissions;
use super::pagecache::{self, PageCacheEntry};
use super::segments::{Segment, SegmentType};


const MAX_SEGMENTS: usize = 6;

static mut KERNEL_ADDRESS_SPACE: Option<SharableVirtualAddressSpace> = None;


pub fn initialize(start: PhysicalAddress, end: PhysicalAddress) -> Result<(), KernelError> {
    pages::init_pages_pool(start, end);
    pagecache::initialize()?;

    let space = VirtualAddressSpace {
        table: TranslationTable::initial_kernel_table(),
        segments: Vec::new(),
        data_index: None,
    };

    unsafe {
        KERNEL_ADDRESS_SPACE = Some(Arc::new(Spinlock::new(space)));
    }
    Ok(())
}

pub type SharableVirtualAddressSpace = Arc<Spinlock<VirtualAddressSpace>>;

pub struct VirtualAddressSpace {
    table: TranslationTable,
    segments: Vec<Segment>,
    data_index: Option<usize>,
}

impl VirtualAddressSpace {
    pub fn get_kernel_space() -> SharableVirtualAddressSpace {
        unsafe {
            KERNEL_ADDRESS_SPACE.clone().unwrap()
        }
    }

    pub fn new() -> Self {
        let pages = pages::get_page_pool();
        let table = TranslationTable::new_table(pages);

        Self {
            table,
            segments: Vec::with_capacity(MAX_SEGMENTS),
            data_index: None,
        }
    }

    pub fn new_sharable() -> SharableVirtualAddressSpace {
        Arc::new(Spinlock::new(Self::new()))
    }

    pub fn translate_addr(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.table.translate_addr(vaddr)
    }

    pub fn add_memory_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) -> Result<(), KernelError> {
        let segment = Segment::new_memory(&mut self.table, permissions, vaddr, vaddr.add(len))?;

        // TODO this is a hack that I'd like to get rid of
        if stype != SegmentType::Stack && (self.data_index.is_none() || vaddr > self.segments[self.data_index.clone().unwrap()].start) {
            self.data_index = Some(self.segments.len());
        }

        self.segments.push(segment);
        Ok(())
    }

    pub fn add_file_backed_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize, vaddr: VirtualAddress, mem_offset: usize, mem_size: usize) -> Result<(), KernelError> {
        let segment = Segment::new_file_backed(&mut self.table, cache, file_offset, file_size, permissions, mem_offset, vaddr, vaddr.add(mem_size).add(mem_offset).align_up(mmu::page_size()))?;

        // TODO this is a hack that I'd like to get rid of
        if stype != SegmentType::Stack && (self.data_index.is_none() || vaddr > self.segments[self.data_index.clone().unwrap()].start) {
            self.data_index = Some(self.segments.len());
        }

        self.segments.push(segment);
        Ok(())
    }

    pub fn clear_segments(&mut self) -> Result<(), KernelError> {
        for i in 0..self.segments.len() {
            self.segments[i].unmap(&mut self.table)?;
        }
        self.segments.clear();
        Ok(())
    }

    pub fn copy_segments(&mut self, parent: &mut Self) -> Result<(), KernelError> {
        for segment in parent.segments.iter() {
            //crate::debug!("cloning segment {:x} to {:x}", usize::from(segment.start), usize::from(segment.end));
            let new_segment = segment.copy(&mut self.table, &mut parent.table)?;
            self.segments.push(new_segment);
        }
        Ok(())
    }

    pub fn adjust_stack_break(&mut self, increment: isize) -> Result<VirtualAddress, KernelError> {
        trace!("vmalloc: adjusting sbrk size by {}", increment);
        if let Some(data_index) = self.data_index {
            let previous_end = self.segments[data_index].end;
            self.segments[data_index].resize(&mut self.table, increment)?;
            Ok(previous_end)
        } else {
            Err(KernelError::NoSegmentFound)
        }
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.table.get_ttbr()
    }

    pub(crate) fn alloc_page_at(&mut self, far: VirtualAddress) -> Result<(), KernelError> {
        for segment in &self.segments {
            if segment.match_range(far) {
                let page_vaddr = far.align_down(mmu::page_size());
                segment.load_page_at(&mut self.table, page_vaddr).unwrap();
                return Ok(());
            }
        }

        Err(KernelError::NoSegmentFound)
    }

    pub(crate) fn copy_on_write_at(&mut self, far: VirtualAddress) -> Result<(), KernelError> {
        let page_vaddr = far.align_down(mmu::page_size());
        let (page, previous_cow) = self.table.reset_page_copy_on_write(page_vaddr).unwrap();
        if previous_cow {
            trace!("copying page on write {:?}", page);
            let pages = pages::get_page_pool();

            // Allocate new page and map it in the current address space
            let new_page = pages.alloc_page_zeroed();
            self.table.update_page_addr(page_vaddr, new_page).unwrap();

            // Copy data into new page
            let page_buffer = mmu::get_page_slice(page);
            let new_page_buffer = mmu::get_page_slice(new_page);
            for i in 0..page_buffer.len() {
                new_page_buffer[i] = page_buffer[i];
            }

            // Decrement the page reference
            pages.free_page(page);

            Ok(())
        } else {
            Err(KernelError::MemoryPermissionDenied)
        }
    }
}

