
use core::arch::asm;

use crate::printkln;
use crate::errors::KernelError;

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

// TODO this isn't actually used by the startup code yet
#[no_mangle]
pub static DEFAULT_TCR: i64 = 0
    | (0b101 << 32)     // 48-bit 1TB Physical Address Size
    | (0b10 << 30)      // 4KB Granuale Size for EL1
    | (0b00 << 14)      // 4KB Granuale Size for EL0
    | (64 - 42);        // Number of unmapped address bits (42-bit addressing assumed)


#[repr(C)]
pub struct TranslationTable(*mut u64);


#[inline(always)]
pub const fn page_size() -> usize {
    4096
}

impl TranslationTable {
    pub fn new_user_table(pages: &mut PageRegion) -> Self {
        let tl0: *mut u64 = pages.alloc_page_zeroed().cast();
        Self(tl0)
    }

    pub fn map_addr(&self, vaddr: *mut u8, paddr: *mut u8, mut len: usize, pages: &mut PageRegion) -> Result<(), KernelError> {
        let tl0_addr_bits = 9 + 9 + 9 + 12;

        let mut paddr = paddr as u64;
        let mut vaddr = vaddr as u64;
        map_level(tl0_addr_bits, self.0, &mut len, &mut vaddr, &mut paddr, pages)
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.0 as u64
    }
}

fn map_level(addr_bits: usize, table: *mut u64, len: &mut usize, vaddr: &mut u64, paddr: &mut u64, pages: &mut PageRegion) -> Result<(), KernelError> {
    let granuale_size = 1 << addr_bits;

    while *len > 0 {
        let mut index = table_index_from_vaddr(addr_bits, *vaddr);
        if (*vaddr & (granuale_size as u64 - 1)) == 0 && *len >= granuale_size {
            map_granuales(addr_bits, table, &mut index, len, vaddr, paddr)?;
            break;
        }

        if addr_bits == 12 {
            break;
        }

        ensure_table_entry(table, index, pages)?;

        map_level(addr_bits - 9, table_ptr_at_index(table, index), len, vaddr, paddr, pages);

        index += 1;
    }

    Ok(())
}

fn map_granuales(addr_bits: usize, table: *mut u64, index: &mut isize, len: &mut usize, vaddr: &mut u64, paddr: &mut u64) -> Result<(), KernelError> {
    let granuale_size = 1 << addr_bits;
    let block_flag = if addr_bits == 12 { TT3_DESCRIPTOR_BLOCK } else { TT2_DESCRIPTOR_BLOCK };

    while *len >= granuale_size {
        if descriptor_type(table, *index) != TT_DESCRIPTOR_EMPTY {
            return Err(KernelError::AddressAlreadyMapped);
        }

        unsafe {
            *table.offset(*index) = (*paddr & TT_BLOCK_MASK) | TT_ACCESS_FLAG | block_flag;
        }

        *index += 1;
        *vaddr += granuale_size as u64;
        *paddr += granuale_size as u64;
        *len -= granuale_size;

        if *index >= page_size() as isize / 8 {
            // If we've reached the end of this table, then return to allow a higher level to increment its index
            break;
        }
    }

    Ok(())
}

fn ensure_table_entry(table: *mut u64, index: isize, pages: &mut PageRegion) -> Result<(), KernelError> {
    let desc_type = descriptor_type(table, index);

    match desc_type {
        TT2_DESCRIPTOR_TABLE => {
            // Do nothing. Sub-table is already present
            Ok(())
        },

        TT_DESCRIPTOR_EMPTY => {
            let next_table: *mut u64 = pages.alloc_page_zeroed().cast();
            unsafe {
                *table.offset(index) = (next_table as u64 & TT_TABLE_MASK) | TT2_DESCRIPTOR_TABLE;
            }
            Ok(())
        },
        TT2_DESCRIPTOR_BLOCK => {
            //panic!("Error already mapped");
            Err(KernelError::AddressAlreadyMapped)
        },
        _ => {
            //panic!("Error corrupted page table");
            Err(KernelError::CorruptTranslationTable)
        },
    }
}


fn table_index_from_vaddr(bits: usize, vaddr: u64) -> isize {
    (((vaddr as u64) >> bits) & 0x1ff) as isize
}

fn table_ptr_at_index(table: *mut u64, index: isize) -> *mut u64 {
    unsafe {
        table_ptr(*table.offset(index))
    }
}

fn table_ptr(descriptor: u64) -> *mut u64 {
    (descriptor & TT_TABLE_MASK) as *mut u64
}

fn block_ptr(descriptor: u64) -> *mut u64 {
    (descriptor & TT_BLOCK_MASK) as *mut u64
}

fn descriptor_type(table: *mut u64, index: isize) -> u64 {
    unsafe {
        *table.offset(index) & TT_TYPE_MASK
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

