
use core::slice;

use alloc::vec::Vec;

use crate::printkln;
use crate::arch::mmu;
use crate::arch::types::PhysicalAddress;
use crate::misc::bitmap::Bitmap;
use crate::misc::ceiling_div;


pub struct PagePool {
    regions: Vec<PageRegion>,
}

pub struct PageRegion {
    bitmap: Bitmap<'static>,
    pages_start: PhysicalAddress,
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
                return addr;
            }
        }
        panic!("Out of memory");
    }

    pub fn alloc_page_zeroed(&mut self) -> PhysicalAddress {
        let paddr = self.alloc_page();
        //crate::printkln!("allocate page {:x}", usize::from(paddr));
        unsafe {
            zero_page(paddr);
        }
        paddr
    }

    pub fn free_page(&mut self, ptr: PhysicalAddress) {
        for region in &mut self.regions {
            if ptr >= region.pages_start && ptr <= region.pages_start.add(region.total_pages() * mmu::page_size()) {
                //crate::printkln!("free page {:x}", usize::from(ptr));
                return region.free_page(ptr);
            }
        }
    }
}

impl PageRegion {
    fn new(start: PhysicalAddress, end: PhysicalAddress) -> Self {
        let page_size = mmu::page_size();
        let total_size = usize::from(end) - usize::from(start);
        let total_pages = total_size / page_size;
        let table_pages = ceiling_div(total_pages / 8, page_size) as usize;

        printkln!("virtual memory: using region at {:?}, size {} MiB, pages {}", start, total_size / 1024 / 1024, total_pages - table_pages);

        let pages = total_pages - table_pages;
        let table: &'static mut [u8] = unsafe { slice::from_raw_parts_mut(start.to_kernel_addr().as_mut(), table_pages * page_size) };
        let pages_start = PhysicalAddress::from(start).add(table_pages * page_size);

        Self {
            bitmap: Bitmap::new(pages, table),
            pages_start,
        }
    }

    pub fn alloc_page(&mut self) -> Option<PhysicalAddress> {
        let bit = self.bitmap.alloc()?;
        let page_addr = self.pages_start.add(bit * mmu::page_size());
        Some(page_addr)
    }

    pub fn free_page(&mut self, ptr: PhysicalAddress) {
        let bit = (usize::from(ptr) - usize::from(self.pages_start)) / mmu::page_size();
        self.bitmap.free(bit);
    }

    pub const fn total_pages(&self) -> usize {
        self.bitmap.total_bits()
    }
}

unsafe fn zero_page(paddr: PhysicalAddress) {
    let page = slice::from_raw_parts_mut(paddr.to_kernel_addr().as_mut(), mmu::page_size());
    for ptr in page.iter_mut() {
        *ptr = 0;
    }
}

