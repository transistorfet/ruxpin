
use core::slice;

use crate::printkln;
use crate::arch::mmu;


//struct PagePool {
//    // TODO a list of all regions
//}

pub struct PageRegion {
    pages: usize,
    pages_free: usize,
    table: &'static mut [u8],
    last_index: usize,
    pages_start: *mut u8,
}

//static mut MEMORY_AREAS: [Option<PagePool>; 20] = unsafe { mem::MaybeUninit::uninit().assume_init() };

static mut PAGES: Option<PageRegion> = None;


pub fn init_pages_area(start: *mut u8, end: *mut u8) {
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
    pub fn alloc_page(&mut self) -> *mut u8 {
        let bit = self.bit_alloc();
        unsafe {
            self.pages_start.add(bit * mmu::page_size())
        }
    }

    pub fn alloc_page_zeroed(&mut self) -> *mut u8 {
        let ptr = self.alloc_page();

        unsafe {
            for i in 0..mmu::page_size() {
                *ptr.add(i) = 0;
            }
        }

        ptr
    }

    pub fn free_page(&mut self, ptr: *mut u8) {
        let bit = (ptr as usize - self.pages_start as usize) / mmu::page_size();
        self.bit_free(bit);
    }

    fn new(start: *mut u8, end: *mut u8) -> Self {
        let page_size = mmu::page_size();
        let total_size = end as usize - start as usize;
        let total_pages = total_size / page_size;
        let table_size = total_pages / 8 / page_size + (total_pages / 8 % page_size != 0) as usize;

        printkln!("virtual memory: using region at {:#x}, size {} MiB, pages {}", start as u64, total_size / 1024 / 1024, total_pages - table_size);

        let pages = total_pages - table_size;
        let table: &'static mut [u8] = unsafe { slice::from_raw_parts_mut(start, table_size * page_size) };
        let pages_start = unsafe { start.add(table_size * page_size) };

        for i in 0..(pages / 8) {
            table[i] = 0;
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

