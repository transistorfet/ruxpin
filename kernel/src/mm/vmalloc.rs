
use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::trace;
use crate::mm::pages;
use crate::misc::align_up;
use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::mm::{MemoryType, MemoryPermissions};
use crate::mm::pagecache::{self, PageCacheEntry};
use crate::mm::segments::{Segment, ArcSegment, SegmentType};
use crate::arch::mmu::{self, TranslationTable};
use crate::arch::types::{VirtualAddress, PhysicalAddress};


const MAX_SEGMENTS: usize = 6;

static mut KERNEL_ADDRESS_SPACE: Option<SharableVirtualAddressSpace> = None;


pub fn initialize(start: PhysicalAddress, end: PhysicalAddress) -> Result<(), KernelError> {
    pages::init_pages_pool(start, end);
    pagecache::initialize()?;

    let space = VirtualAddressSpace {
        table: TranslationTable::initial_kernel_table(),
        segments: Vec::new(),
        data: None,
    };

    unsafe {
        KERNEL_ADDRESS_SPACE = Some(Arc::new(Spinlock::new(space)));
    }
    Ok(())
}

pub type SharableVirtualAddressSpace = Arc<Spinlock<VirtualAddressSpace>>;

pub struct VirtualAddressSpace {
    table: TranslationTable,
    segments: Vec<ArcSegment>,
    data: Option<ArcSegment>,
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
            data: None,
        }
    }

    pub fn new_sharable() -> SharableVirtualAddressSpace {
        Arc::new(Spinlock::new(Self::new()))
    }

    pub fn translate_addr(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        self.table.translate_addr(vaddr)
    }

    pub fn add_memory_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, vaddr: VirtualAddress, len: usize) -> Result<(), KernelError> {
        let segment = Arc::new(Spinlock::new(Segment::new_memory(permissions, vaddr, vaddr.add(len))?));
        if stype != SegmentType::Stack && (self.data.is_none() || vaddr > self.data.as_mut().unwrap().lock().start) {
            self.data = Some(segment.clone());
        }
        self.segments.push(segment);

        let pages = pages::get_page_pool();
        self.table.map_paged_range(MemoryType::Unallocated, permissions, vaddr, align_up(len, mmu::page_size()), pages)?;
        Ok(())
    }

    pub fn add_file_backed_segment(&mut self, stype: SegmentType, permissions: MemoryPermissions, cache: Arc<PageCacheEntry>, file_offset: usize, file_size: usize, vaddr: VirtualAddress, mem_offset: usize, mem_size: usize) -> Result<(), KernelError> {
        let segment = Arc::new(Spinlock::new(Segment::new_file_backed(cache, file_offset, file_size, permissions, mem_offset, vaddr, vaddr.add(mem_size).add(mem_offset).align_up(mmu::page_size()))?));
        if stype != SegmentType::Stack && (self.data.is_none() || vaddr > self.data.as_mut().unwrap().lock().start) {
            self.data = Some(segment.clone());
        }
        self.segments.push(segment);

        let pages = pages::get_page_pool();
        self.table.map_paged_range(MemoryType::Unallocated, permissions, vaddr, align_up(mem_size + mem_offset, mmu::page_size()), pages)?;
        Ok(())
    }

    pub fn clear_segments(&mut self) -> Result<(), KernelError> {
        let pages = pages::get_page_pool();

        for i in 0..self.segments.len() {
            let start = self.segments[i].lock().start;
            let len = self.segments[i].lock().page_aligned_len();
            self.table.unmap_range(start, len, pages)?
        }
        self.segments.clear();
        Ok(())
    }

    pub fn copy_segments(&mut self, parent: &mut Self) -> Result<(), KernelError> {
        for segment in parent.segments.iter() {
            //crate::debug!("cloning segment {:x} to {:x}", usize::from(segment.start), usize::from(segment.end));
            self.segments.push(segment.clone());

            let segment = &*segment.lock();
            let pages = pages::get_page_pool();

            if segment.permissions == MemoryPermissions::ReadWrite {
                self.table.remap_range_copy_on_write(&mut parent.table, segment.start, segment.page_aligned_len(), pages)?;
            } else  {
                self.table.duplicate_paged_range(&mut parent.table, segment.permissions, segment.start, segment.page_aligned_len(), pages)?;
            }
        }
        Ok(())
    }

    // TODO technically increment should be isize, and can be negative to shrink the size
    pub fn adjust_stack_break(&mut self, increment: usize) -> Result<VirtualAddress, KernelError> {
        trace!("vmalloc: adjusting sbrk size by {}", increment);
        let inc_aligned = align_up(increment, mmu::page_size());
        let segment = self.data.clone().unwrap();
        let mut locked_seg = segment.try_lock()?;
        let previous_end = locked_seg.end;
        locked_seg.end = locked_seg.end.add(inc_aligned);

        let pages = pages::get_page_pool();
        self.table.map_paged_range(MemoryType::Unallocated, locked_seg.permissions, previous_end, inc_aligned, pages)?;
        Ok(previous_end)
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.table.get_ttbr()
    }

    pub(crate) fn alloc_page_at(&mut self, far: VirtualAddress) -> Result<(), KernelError> {
        for segment in &self.segments {
            let locked_seg = segment.try_lock()?;
            if locked_seg.match_range(far) {
                let page_vaddr = far.align_down(mmu::page_size());
                locked_seg.load_page_at(&mut self.table, page_vaddr).unwrap();
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

