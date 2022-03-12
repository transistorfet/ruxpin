
use core::arch::asm;

use crate::printkln;

// TODO this should be changed to PagePool when you decide how you'd like to do it
use crate::mm::vmalloc::PageRegion;


#[allow(dead_code)]
const TT_DESCRIPTOR_EMPTY: u64 = 0b00;
#[allow(dead_code)]
const TT2_DESCRIPTOR_TABLE: u64 = 0b11;
#[allow(dead_code)]
const TT2_DESCRIPTOR_BLOCK: u64 = 0b01;
const TT3_DESCRIPTOR_BLOCK: u64 = 0b11;

#[allow(dead_code)]
const TT_ACCESS_FLAG: u64 = 1 << 10;

#[allow(dead_code)]
const TT_TYPE_MASK: u64 = 0b11;
const TT_TABLE_MASK: u64 = 0x0000_ffff_ffff_ffff_f000;
const TT_BLOCK_MASK: u64 = 0x0000_ffff_ffff_ffff_f000;

extern {
    static _kernel_translation_table_l0: [u64; page_size()];
}


#[repr(C)]
pub struct TranslationTable(*mut u64);


pub fn init_mmu(pages: &mut PageRegion) {
    let tl0: *mut u64 = pages.alloc_page_zeroed().cast();
    let tl1: *mut u64 = pages.alloc_page_zeroed().cast();

    unsafe {
        (*tl0) = (tl1 as u64) | 0b11;
        // Map 1GB space directly to the same addresses
        (*tl1) = 0 | (0b1 << 10) | 0b01;

        //enable_mmu(_DEFAULT_TRANSLATION_TABLE_L0 as *mut u8, tl0 as *mut u8);
    }
}

unsafe fn enable_mmu(kernel: *mut u8, user: *mut u8) {
    let tcr: i64 = 0
    | (0b101 << 32)     // 48-bit 1TB Physical Address Size
    | (0b10 << 30)      // 4KB Granuale Size for EL1
    | (0b00 << 14)      // 4KB Granuale Size for EL0
    | (64 - 42);        // Number of unmapped address bits (42-bit addressing assumed)

    asm!(
        //"msr    TTBR1_EL1, {kernel}",
        //"msr    TTBR0_EL1, {user}",
        //"msr    TCR_EL1, {tcr}",
        //"isb",
        "mrs    {tmp}, SCTLR_EL1",
        "orr    {tmp}, {tmp}, 1",
        "msr    SCTLR_EL1, {tmp}",
        "isb",
        //tcr = in(reg) tcr,
        //kernel = in(reg) kernel,
        //user = in(reg) user,
        tmp = out(reg) _,
    );
}

#[inline(always)]
pub const fn page_size() -> usize {
    4096
}

impl TranslationTable {
    pub fn new_user_table(pages: &mut PageRegion) -> Self {
        let tl0: *mut u64 = pages.alloc_page_zeroed().cast();
        Self(tl0)
    }

    pub fn map_addr(&self, vaddr: *mut u8, paddr: *mut u8, len: usize, pages: &mut PageRegion) {
        // Index Table Level 0
        let tl0_index = (vaddr as u64) >> (9 + 9 + 9 + 12) & 0x1ff;
        let tl0_entry = unsafe { self.0.offset(tl0_index as isize) };

        ensure_table_entry(tl0_entry, pages);

        // Index Table Level 1
        let tl1_index = (vaddr as u64) >> (9 + 9 + 12) & 0x1ff;
        let tl1_entry = unsafe {((*tl0_entry & TT_TABLE_MASK) as *mut u64).offset(tl1_index as isize) };


        if len >> (9 + 9 + 12) != 0 {
            // big segment
        }

        ensure_table_entry(tl1_entry, pages);

        // Index Table Level 2
        let tl2_index = (vaddr as u64) >> (9 + 12) & 0x1ff;
        let tl2_entry = unsafe {((*tl1_entry & TT_TABLE_MASK) as *mut u64).offset(tl2_index as isize) };

        if len >> (9 + 12) != 0 {
            // big segment
        }

        ensure_table_entry(tl2_entry, pages);

        // Index Table Level 3
        let tl3_index = (vaddr as u64) >> 12 & 0x1ff;
        let tl3_entry = unsafe {((*tl2_entry & TT_TABLE_MASK) as *mut u64).offset(tl3_index as isize) };

        map_granuales(tl3_entry, paddr, page_size(), ceiling_div(len, page_size()));
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.0 as u64
    }
}

fn ensure_table_entry(parent_entry: *mut u64, pages: &mut PageRegion) {
    let is_empty = unsafe { *parent_entry } & TT_TYPE_MASK == TT_DESCRIPTOR_EMPTY;
    if is_empty {
        let next_table: *mut u64 = pages.alloc_page_zeroed().cast();
        unsafe {
            *parent_entry = (next_table as u64 & TT_TABLE_MASK) | TT2_DESCRIPTOR_TABLE;
        }
    }
}

fn map_granuales(table: *mut u64, paddr: *mut u8, granuale_size: usize, granuales: usize) {
    for granuale in 0..granuales {
        unsafe {
            *table.offset(granuale as isize) = (paddr.offset((granuale * granuale_size) as isize) as u64) & TT_BLOCK_MASK | TT_ACCESS_FLAG | TT3_DESCRIPTOR_BLOCK;
        }
    }
}

fn desc_to_table(entry: *mut u64) -> *mut u64 {
    unsafe {
        (*entry & TT_TABLE_MASK) as *mut u64
    }
}

fn ceiling_div(size: usize, units: usize) -> usize {
    (size / units) + (size % units != 0) as usize
}

