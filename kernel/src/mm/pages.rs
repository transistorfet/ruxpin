
use core::slice;

use crate::printkln;
use crate::arch::mmu;
use crate::arch::types::PhysicalAddress;

//struct PagePool {
//    // TODO a list of all regions
//}

pub struct PageRegion {
    pages: usize,
    pages_free: usize,
    table: &'static mut [u8],
    last_index: usize,
    pages_start: PhysicalAddress,
}

static mut PAGES: Option<PageRegion> = None;


pub fn init_pages_area(start: PhysicalAddress, end: PhysicalAddress) {
    let pages = PageRegion::new(start, end);

    unsafe {
        PAGES = Some(pages);
    }
}

pub fn get_page_area<'a>() -> &'a mut PageRegion {
    unsafe {
        PAGES.as_mut().unwrap()
    }
}

impl PageRegion {
    pub fn alloc_page(&mut self) -> PhysicalAddress {
        let bit = self.bit_alloc();
        let page_addr = self.pages_start.add(bit * mmu::page_size());
        page_addr
    }

    pub fn alloc_page_zeroed(&mut self) -> PhysicalAddress {
        let paddr = self.alloc_page();

        unsafe {
            let page = slice::from_raw_parts_mut(paddr.as_ptr(), mmu::page_size());
            for ptr in page.iter_mut() {
                *ptr = 0;
            }
        }

        paddr
    }

    pub fn free_page(&mut self, ptr: PhysicalAddress) {
        let bit = (usize::from(ptr) - usize::from(self.pages_start)) / mmu::page_size();
        self.bit_free(bit);
    }

    fn new(start: PhysicalAddress, end: PhysicalAddress) -> Self {
        let page_size = mmu::page_size();
        let total_size = usize::from(end) - usize::from(start);
        let total_pages = total_size / page_size;
        let table_size = total_pages / 8 / page_size + (total_pages / 8 % page_size != 0) as usize;

        printkln!("virtual memory: using region at {:?}, size {} MiB, pages {}", start, total_size / 1024 / 1024, total_pages - table_size);

        let pages = total_pages - table_size;
        let table: &'static mut [u8] = unsafe { slice::from_raw_parts_mut(start.as_ptr(), table_size * page_size / 8) };
        let pages_start = PhysicalAddress::from(start).add(table_size * page_size);

        for byte in table.iter_mut() {
            *byte = 0;
        }

        Self {
            pages,
            pages_free: pages,
            table,
            last_index: 0,
            pages_start,
        }
    }

    fn bit_alloc(&mut self) -> usize {
        let mut i = self.last_index;

        loop {
            if i >= ceiling_div(self.pages, 8) {
                i = 0;
            }

            if self.table[i] != 0xff {
                let mut bit = 0;
                while bit < 7 && (self.table[i] & (0x01 << bit)) != 0 {
                    bit += 1;
                }
                self.table[i] |= 0x01 << bit;
                self.last_index = i;
                self.pages_free -= 1;
                return (i * 8) + bit;
            }

            i += 1;
            if i == self.last_index {
                panic!("Out of memory");
            }
        }
    }

    fn bit_free(&mut self, bitnum: usize) {
        let i = bitnum >> 3;
        let bit = bitnum & 0x7;
        self.table[i] &= !(0x01 << bit);
        self.pages_free += 1;
        // NOTE we could set last_index here, but not doing that might mean more contiguous chunks get allocated
    }
}

fn ceiling_div(size: usize, units: usize) -> usize {
    (size / units) + (size % units != 0) as usize
}

