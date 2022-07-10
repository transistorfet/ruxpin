
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
        }
    }

    pub fn new_sharable() -> SharableVirtualAddressSpace {
        Arc::new(Spinlock::new(Self::new()))
    }

    pub fn translate_addr(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.table.translate_addr(vaddr)
    }

    pub fn add_memory_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) -> Result<(), KernelError> {
        let segment = Segment::new_memory(&mut self.table, stype, permissions, vaddr, vaddr.add(len))?;

        self.insert_segment(segment);
        Ok(())
    }

    pub fn add_file_backed_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize, vaddr: VirtualAddress, mem_offset: usize, mem_size: usize) -> Result<(), KernelError> {
        let vaddr_end = vaddr.add(mem_size).add(mem_offset).align_up(mmu::page_size());
        let segment = Segment::new_file_backed(&mut self.table, stype, permissions, mem_offset, vaddr, vaddr_end, cache, file_offset, file_size)?;

        self.insert_segment(segment);
        Ok(())
    }

    fn insert_segment(&mut self, segment: Segment) {
        let i = self.segments.iter().position(|seg| segment.start < seg.start).unwrap_or(self.segments.len());
        self.segments.insert(i, segment);
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
        let segs = &mut self.segments;
        let stack = segs.len() - 1;
        let data = segs.len() - 2;

        if segs.len() < 2 && segs[stack].stype != SegmentType::Stack || segs[data].stype != SegmentType::Data {
            return Err(KernelError::NoSegmentFound);
        }

        let previous_end = segs[data].end;
        if increment != 0 {
            if
                (increment >= 0 && segs[data].end.add(increment as usize) >= segs[stack].start) ||
                (increment >= 0 && segs[data].end.sub((-1 * increment) as usize) >= segs[stack].start)
            {
                segs[stack].resize_stack(&mut self.table, increment)?;
            }
            segs[data].resize(&mut self.table, increment)?;
        }

        Ok(previous_end)
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.table.get_ttbr()
    }

    pub(crate) fn alloc_page_at(&mut self, fault_addr: VirtualAddress) -> Result<(), KernelError> {
        let page_vaddr = fault_addr.align_down(mmu::page_size());

        for segment in &self.segments {
            if segment.match_range(fault_addr) {
                segment.load_page_at(&mut self.table, page_vaddr)?;
                return Ok(());
            }
        }

        Err(KernelError::NoSegmentFound)
    }

    pub(crate) fn copy_on_write_at(&mut self, fault_addr: VirtualAddress) -> Result<(), KernelError> {
        let page_vaddr = fault_addr.align_down(mmu::page_size());
        let (page, previous_cow) = self.table.reset_page_copy_on_write(page_vaddr)?;
        if previous_cow {
            trace!("copying page on write {:?}", page);
            let pages = pages::get_page_pool();

            // Allocate new page and map it in the current address space
            let new_page = pages.alloc_page_zeroed();
            self.table.update_page_addr(page_vaddr, new_page, pages)?;

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

