
use core::mem;
use core::ptr;
use core::slice;

use alloc::vec::Vec;

use crate::printkln;
use crate::arch::mmu;
use crate::arch::types::PhysicalAddress;
use crate::misc::{ceiling_div, align_up};


const BITS_PER_ALLOC: usize = 32;

pub struct PagePool {
    regions: Vec<PageRegion>,
}

pub struct PageRegion {
    total_pages: usize,
    free_pages: usize,
    last_index: usize,
    alloc_table: &'static mut [u32],
    desc_table: &'static mut [Page],
    pages_start: PhysicalAddress,
}

pub type PageRefCount = u16;

pub struct Page {
    refcount: PageRefCount,
}

static mut PAGES: PagePool = PagePool::new();


pub fn init_pages_area(start: PhysicalAddress, end: PhysicalAddress) {
    let pages = PageRegion::new(start, end);

    unsafe {
        PAGES.regions.push(pages);
    }
}

pub fn get_page_area<'a>() -> &'a mut PagePool {
    unsafe {
        &mut PAGES
    }
}

impl PagePool {
    pub const fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn alloc_page(&mut self) -> PhysicalAddress {
        for region in &mut self.regions {
            if let Some(addr) = region.alloc_page() {
                //printkln!("pages: allocating page at {:#x}", usize::from(addr));
                return addr;
            }
        }
        panic!("Out of memory");
    }

    pub fn alloc_page_zeroed(&mut self) -> PhysicalAddress {
        let paddr = self.alloc_page();
        unsafe {
            zero_page(paddr);
        }
        paddr
    }

    pub fn free_page(&mut self, ptr: PhysicalAddress) {
        for region in &mut self.regions {
            if ptr >= region.pages_start && ptr <= region.pages_start.add(region.total_pages() * mmu::page_size()) {
                //printkln!("pages: freeing page at {:x}", usize::from(ptr));
                region.free_page(ptr);
                return;
            }
        }
        panic!("pages: attempting to free a page with no region: {:x}", usize::from(ptr));
    }

    pub fn ref_page(&mut self, ptr: PhysicalAddress) -> PhysicalAddress {
        for region in &mut self.regions {
            if ptr >= region.pages_start && ptr <= region.pages_start.add(region.total_pages() * mmu::page_size()) {
                printkln!("pages: incrementing page ref at {:#x}", usize::from(ptr));
                return region.ref_page(ptr);
            }
        }
        panic!("pages: attempting to reference a page with no region: {:x}", usize::from(ptr));
    }
}

impl PageRegion {
    fn new(start: PhysicalAddress, end: PhysicalAddress) -> Self {
        let page_size = mmu::page_size();
        let total_size = usize::from(end) - usize::from(start);
        let total_pages = total_size / page_size;

        let alloc_table_size = align_up(size_of_alloc_table(total_pages), 8);
        let desc_table_size = align_up(size_of_desc_table(total_pages), 8);
        let total_table_size = alloc_table_size + desc_table_size;
        let total_table_pages = ceiling_div(total_table_size, page_size);
        let usable_pages = total_pages - total_table_pages;

        printkln!("virtual memory: using region at {:?}, size {} MiB, pages {}", start, total_size / 1024 / 1024, usable_pages);
        printkln!("using {} pages ({} bytes) for descriptors (ratio of {})", total_table_pages, total_table_size, total_pages / total_table_pages);
        printkln!("alloc {} {}, desc {} {}", alloc_table_size, alloc_table_size / page_size, desc_table_size, desc_table_size / page_size);

        let alloc_table = init_alloc_table(start, usable_pages);
        let desc_table = init_desc_table(start.add(alloc_table_size), usable_pages);
        let pages_start = start.add(total_table_size).align_up(page_size);

        Self {
            total_pages: usable_pages,
            free_pages: usable_pages,
            last_index: 0,
            alloc_table,
            desc_table,
            pages_start,
        }
    }

    pub const fn total_pages(&self) -> usize {
        self.total_pages
    }

    pub fn alloc_page(&mut self) -> Option<PhysicalAddress> {
        let bit = self.alloc_bit()?;
        self.desc_table[bit].refcount = 1;
        let page_addr = self.pages_start.add(bit * mmu::page_size());
        Some(page_addr)
    }

    pub fn free_page(&mut self, ptr: PhysicalAddress) {
        //printkln!("pages: decrementing page ref at {:x}", usize::from(ptr));
        let bit = (usize::from(ptr) - usize::from(self.pages_start)) / mmu::page_size();
        self.desc_table[bit].refcount -= 1;
        if self.desc_table[bit].refcount == 0 {
            //printkln!("pages: freeing page at {:x}", usize::from(ptr));
            self.free_bit(bit);
        }
    }

    pub fn ref_page(&mut self, ptr: PhysicalAddress) -> PhysicalAddress {
        let bit = (usize::from(ptr) - usize::from(self.pages_start)) / mmu::page_size();
        self.desc_table[bit].refcount += 1;
        if self.desc_table[bit].refcount == u16::MAX {
            panic!("Error: reference count for page {:?} has reached the limit", ptr);
        }
        ptr
    }

    fn alloc_bit(&mut self) -> Option<usize> {
        let mut i = self.last_index;

        loop {
            if i >= ceiling_div(self.total_pages, BITS_PER_ALLOC) {
                i = 0;
            }

            if !self.alloc_table[i] != 0 {
                let mut bit = 0;
                while bit < (BITS_PER_ALLOC - 1) && (self.alloc_table[i] & (0x01 << bit)) != 0 {
                    bit += 1;
                }
                self.alloc_table[i] |= 0x01 << bit;
                self.last_index = i;
                self.free_pages -= 1;
                return Some((i * BITS_PER_ALLOC) + bit);
            }

            i += 1;
            if i == self.last_index {
                return None;
            }
        }
    }

    fn free_bit(&mut self, bitnum: usize) {
        let i = bitnum / BITS_PER_ALLOC;
        let bit = bitnum & (BITS_PER_ALLOC - 1);
        self.alloc_table[i] &= !(0x01 << bit);
        self.free_pages += 1;
        // NOTE we could set last_index here, but not doing that might mean more contiguous chunks get allocated
        //if i < self.last_index {
        //    self.last_index = i;
        //}
    }
}

unsafe fn zero_page(paddr: PhysicalAddress) {
    let page: &mut [u8] = slice::from_raw_parts_mut(paddr.to_kernel_addr().as_mut(), mmu::page_size());
    for ptr in page.iter_mut() {
        *ptr = 0;
    }
}

fn init_alloc_table(start: PhysicalAddress, pages: usize) -> &'static mut [u32] {
    let alloc_table: &'static mut [u32] = unsafe { slice::from_raw_parts_mut(start.to_kernel_addr().as_mut(), pages / BITS_PER_ALLOC) };
    for alloc in alloc_table.iter_mut() {
        *alloc = 0;
    }
    alloc_table
}

fn size_of_alloc_table(pages: usize) -> usize {
    ceiling_div(pages, 8)
}

fn init_desc_table(start: PhysicalAddress, pages: usize) -> &'static mut [Page] {
    let desc_table: &'static mut [Page] = unsafe { slice::from_raw_parts_mut(start.to_kernel_addr().as_mut(), pages) };
    for page in desc_table.iter_mut() {
        unsafe {
            ptr::write(page, Page {
                refcount: 0
            });
        }
    }
    desc_table
}

fn size_of_desc_table(pages: usize) -> usize {
    pages * mem::size_of::<Page>()
}

