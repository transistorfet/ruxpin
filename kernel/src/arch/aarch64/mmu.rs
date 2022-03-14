
use core::arch::asm;

use crate::printkln;
use crate::mm::MemoryAccess;
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

const TT_ACCESS_FLAG: u64 = 1 << 10;
const TT_READ_ONLY_FLAG: u64 = 0b11 << 6;
const TT_READ_WRITE_FLAG: u64 = 0b01 << 6;
const TT_NO_EXECUTE_FLAG: u64 = 0b11 << 53;

#[allow(dead_code)]
const TT_TYPE_MASK: u64 = 0b11;
const TT_TABLE_MASK: u64 = 0x0000_ffff_ffff_ffff_f000;
const TT_BLOCK_MASK: u64 = 0x0000_ffff_ffff_ffff_f000;

const TL0_ADDR_BITS: usize = 9 + 9 + 9 + 12;

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


pub type VirtualAddress = u64;
pub type PhysicalAddress = u64;

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

    pub fn map_addr(&self, access: MemoryAccess, vaddr: VirtualAddress, paddr: PhysicalAddress, mut len: usize, pages: &mut PageRegion) -> Result<(), KernelError> {
        let flags = memory_access_flags(access);
        let mut paddr = paddr;
        let mut vaddr = vaddr;
        map_level(TL0_ADDR_BITS, self.0, &mut len, &mut vaddr, &mut paddr, flags, pages)
    }

    pub fn translate_addr(&self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        lookup_level(TL0_ADDR_BITS, self.0, vaddr)
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.0 as u64
    }
}

fn map_level(addr_bits: usize, table: *mut u64, len: &mut usize, vaddr: &mut VirtualAddress, paddr: &mut PhysicalAddress, flags: u64, pages: &mut PageRegion) -> Result<(), KernelError> {
    let granuale_size = 1 << addr_bits;

    while *len > 0 {
        let mut index = table_index_from_vaddr(addr_bits, *vaddr);
        if (*vaddr & (granuale_size as u64 - 1)) == 0 && *len >= granuale_size {
            map_granuales(addr_bits, table, &mut index, len, vaddr, paddr, flags)?;
            break;
        }

        if addr_bits == 12 {
            break;
        }

        ensure_table_entry(table, index, pages)?;

        map_level(addr_bits - 9, table_ptr(table, index), len, vaddr, paddr, flags, pages);

        index += 1;
    }

    Ok(())
}

fn map_granuales(addr_bits: usize, table: *mut u64, index: &mut isize, len: &mut usize, vaddr: &mut VirtualAddress, paddr: &mut PhysicalAddress, flags: u64) -> Result<(), KernelError> {
    let granuale_size = 1 << addr_bits;
    let block_flag = if addr_bits == 12 { TT3_DESCRIPTOR_BLOCK } else { TT2_DESCRIPTOR_BLOCK };

    while *len >= granuale_size {
        if descriptor_type(table, *index) != TT_DESCRIPTOR_EMPTY {
            return Err(KernelError::AddressAlreadyMapped);
        }

        unsafe {
            *table.offset(*index) = (*paddr & TT_BLOCK_MASK) | TT_ACCESS_FLAG | flags | block_flag;
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

fn lookup_level(addr_bits: usize, table: *mut u64, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
    let granuale_size = 1 << addr_bits;

    let mut index = table_index_from_vaddr(addr_bits, vaddr);
    if is_block(addr_bits, table, index) {
        Ok(block_ptr(table, index) | (vaddr & (granuale_size - 1)))
    } else if addr_bits == 12 {
        Err(KernelError::AddressUnmapped)
    } else {
        lookup_level(addr_bits - 9, table_ptr(table, index), vaddr)
    }
}


fn table_index_from_vaddr(bits: usize, vaddr: VirtualAddress) -> isize {
    (((vaddr as u64) >> bits) & 0x1ff) as isize
}

fn table_ptr(table: *mut u64, index: isize) -> *mut u64 {
    unsafe {
        (*table.offset(index) & TT_TABLE_MASK) as *mut u64
    }
}

fn block_ptr(table: *mut u64, index: isize) -> PhysicalAddress {
    unsafe {
        (*table.offset(index) & TT_BLOCK_MASK)
    }
}

fn descriptor_type(table: *mut u64, index: isize) -> u64 {
    unsafe {
        *table.offset(index) & TT_TYPE_MASK
    }
}

fn is_block(addr_bits: usize, table: *mut u64, index: isize) -> bool {
    let dtype = descriptor_type(table, index);
    if addr_bits == 12 {
        dtype == TT3_DESCRIPTOR_BLOCK
    } else {
        dtype == TT2_DESCRIPTOR_BLOCK
    }
}

fn memory_access_flags(access: MemoryAccess) -> u64 {
    match access {
        MemoryAccess::ReadOnly => TT_READ_ONLY_FLAG | TT_NO_EXECUTE_FLAG,
        MemoryAccess::ReadExecute => TT_READ_ONLY_FLAG,
        MemoryAccess::ReadWrite => TT_READ_WRITE_FLAG | TT_NO_EXECUTE_FLAG,
        MemoryAccess::ReadWriteExecute => TT_READ_WRITE_FLAG,
    }
}

