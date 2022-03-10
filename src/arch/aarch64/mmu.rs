
use core::arch::asm;

// TODO this should be changed to PagePool when you decide how you'd like to do it
use crate::mm::vmalloc::PageRegion;


const TT_DESCRIPTOR_EMPTY: u64 = 0b00;
const TT_DESCRIPTOR_TABLE: u64 = 0b11;
const TT_DESCRIPTOR_BLOCK: u64 = 0b01;

const TT_ACCESS_FLAG: u64 = 1 << 10;

const TT_TYPE_MASK: u64 = 0b11;
const TT_TABLE_MASK: u64 = 0x0000_ffff_ffff_ffff_f000;

#[repr(C)]
pub struct TranslationTable(u64);

pub fn init_mmu(pages: &mut PageRegion) {
    let tl0: *mut u64 = pages.alloc_page_zeroed().cast();
    let tl1: *mut u64 = pages.alloc_page_zeroed().cast();
    let tl2: *mut u64 = pages.alloc_page_zeroed().cast();

    unsafe {
        (*tl0) = (tl1 as u64) | 0b11;

        (*tl1) = 0 | (0b1 << 10) | 0b01;
        //(*tl1) = (tl2 as u64) | 0b11;

        //for i in 0..512 {
        //    *code_tl2.offset(i) = (i as u64 * 0x20_0000) | (0b1 << 10) | 0b01
        //}

        //(*tl2) = 0 | (0b1 << 10) | 0b01;
        //(*tl2.offset(1)) = 0x20_0000 | (0b1 << 10) | 0b01;
        //(*tl2.offset(0x1f9)) = 0x3F20_0000 | (0b1 << 10) | 0b01;

        enable_mmu(tl0 as *mut u8, tl0 as *mut u8);
    }
}

unsafe fn enable_mmu(kernel: *mut u8, user: *mut u8) {
    let tcr: i64 = 0
    | (0b101 << 32)     // 48-bit 1TB Physical Address Size
    | (0b10 << 30)      // 4KB Granuale Size for EL1
    | (0b00 << 14)      // 4KB Granuale Size for EL0
    | (64 - 42);        // Number of unmapped address bits (42-bit addressing assumed)

    asm!(
        "msr    TTBR1_EL1, {kernel}",
        "msr    TTBR0_EL1, {user}",
        "msr    TCR_EL1, {tcr}",
        "isb",
        "mrs    {tmp}, SCTLR_EL1",
        "orr    {tmp}, {tmp}, 1",
        "msr    SCTLR_EL1, {tmp}",
        "isb",
        tcr = in(reg) tcr,
        kernel = in(reg) kernel,
        user = in(reg) user,
        tmp = out(reg) _,
    );
}

#[inline(always)]
pub fn page_size() -> usize {
    4096
}

impl TranslationTable {
    pub fn map_addr(&self, vaddr: *mut u8, paddr: *mut u8, len: usize) {
        // Index Table Level 0
        let tl0_index = (vaddr as u64) >> (9 + 9 + 12) & 0x1ff;
        let tl0_entry = unsafe { (self.0 as *mut u64).offset(tl0_index as isize) };
        if len >> (9 + 9 + 12) != 0 {
            // big segment
        }

        let is_empty = unsafe { *tl0_entry } & TT_TYPE_MASK == TT_DESCRIPTOR_EMPTY;
        if is_empty {
            // TODO allocate table
        }

        // Index Table Level 1
        let tl1_index = (vaddr as u64) >> (9 + 12) & 0x1ff;
        let tl1_entry = unsafe {(self.0 as *mut u64).offset(tl1_index as isize) };

        if len >> (9 + 12) != 0 {
            // big segment
        }

        let is_empty = unsafe { *tl1_entry } & TT_TYPE_MASK == TT_DESCRIPTOR_EMPTY;
        if is_empty {
            // TODO allocate table
        }

        // Index Table Level 2
        let tl2_index = (vaddr as u64) >> 12 & 0x1ff;
        let tl2_entry = unsafe {(self.0 as *mut u64).offset(tl2_index as isize) };

        for page in 0..pages(len) {
            unsafe {
                *tl2_entry.offset(page as isize) = (paddr.offset((page * page_size()) as isize) as u64) & TT_TABLE_MASK | TT_ACCESS_FLAG | TT_DESCRIPTOR_BLOCK;
            }
        }
    }
}

fn desc_to_table(entry: *mut u64) -> *mut u64 {
    unsafe {
        (*entry & TT_TABLE_MASK) as *mut u64
    }
}

fn pages(size: usize) -> usize {
    (size / page_size()) + (size % page_size() != 0) as usize
}

