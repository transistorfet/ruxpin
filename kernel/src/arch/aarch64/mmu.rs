
use core::slice;

use crate::errors::KernelError;
use crate::mm::pages::PagePool;
use crate::mm::{MemoryType, MemoryPermissions};

use super::types::{PhysicalAddress, VirtualAddress};


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
const TT_TABLE_MASK: u64 = 0x0000_ffff_ffff_f000;
const TT_BLOCK_MASK: u64 = 0x0000_ffff_ffff_f000;

const TL0_ADDR_BITS: usize = 9 + 9 + 9 + 12;

extern "C" {
    static _kernel_translation_table_l0: [u64; page_size()];
}

// TODO this isn't actually used by the startup code yet
#[no_mangle]
pub static DEFAULT_TCR: i64 =
      (0b101 << 32)     // 48-bit 1TB Physical Address Size
    | (0b10 << 30)      // 4KB Granuale Size for EL1
    | (0b00 << 14)      // 4KB Granuale Size for EL0
    | (64 - 42);        // Number of unmapped address bits (42-bit addressing assumed)



pub struct TranslationTable(u64);


#[inline(always)]
pub const fn page_size() -> usize {
    4096
}

#[inline(always)]
pub const fn table_entries() -> usize {
    page_size() / 8
}

impl TranslationTable {
    pub fn new_user_table(pages: &mut PagePool) -> Self {
        let tl0 = allocate_table(pages);
        Self(u64::from(tl0))
    }

    pub fn map_addr<F>(&mut self, mtype: MemoryType, access: MemoryPermissions, mut vaddr: VirtualAddress, mut len: usize, pages: &mut PagePool, map_block: &F) -> Result<(), KernelError>
    where
        F: Fn(&mut PagePool, VirtualAddress, usize) -> Option<PhysicalAddress>
    {
        let flags = memory_type_flags(mtype) | memory_permissions_flags(access);
        map_level(TL0_ADDR_BITS, self.as_slice(), &mut len, &mut vaddr, flags, pages, map_block)
    }

    pub fn unmap_addr<F>(&mut self, mut vaddr: VirtualAddress, mut len: usize, pages: &mut PagePool, unmap_block: &F) -> Result<(), KernelError>
    where
        F: Fn(&mut PagePool, VirtualAddress, PhysicalAddress)
    {
        unmap_level(TL0_ADDR_BITS, self.as_slice(), &mut len, &mut vaddr, pages, unmap_block)
    }

    pub fn translate_addr(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        let (descriptor, granuale_size) = lookup_level(TL0_ADDR_BITS, self.as_slice(), vaddr)?;
        Ok(PhysicalAddress::from(*descriptor & TT_BLOCK_MASK).add(usize::from(vaddr) & (granuale_size - 1)))
    }

    pub fn update_mapping(&mut self, vaddr: VirtualAddress, paddr: PhysicalAddress, expected_size: usize) -> Result<(), KernelError> {
        let (descriptor, granuale_size) = lookup_level(TL0_ADDR_BITS, self.as_slice(), vaddr)?;
        if granuale_size == expected_size {
            *descriptor &= !TT_BLOCK_MASK;
            *descriptor |= (u64::from(paddr) & TT_BLOCK_MASK) | TT_ACCESS_FLAG;
            Ok(())
        } else {
            Err(KernelError::UnexpectedGranualeSize)
        }
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.0 as u64
    }

    fn as_slice(&mut self) -> &mut [u64] {
        table_as_slice(PhysicalAddress::from(self.0))
    }
}

fn map_level<F>(addr_bits: usize, table: &mut [u64], len: &mut usize, vaddr: &mut VirtualAddress, flags: u64, pages: &mut PagePool, map_block: &F) -> Result<(), KernelError>
where
    F: Fn(&mut PagePool, VirtualAddress, usize) -> Option<PhysicalAddress>
{
    let granuale_size = 1 << addr_bits;

    while *len > 0 {
        let mut index = table_index_from_vaddr(addr_bits, *vaddr);
        if (usize::from(*vaddr) & (granuale_size - 1)) == 0 && *len >= granuale_size {
            let should_break = map_granuales(addr_bits, table, &mut index, len, vaddr, flags, pages, map_block)?;
            if should_break {
                break;
            }
        }

        if addr_bits == 12 {
            break;
        }

        ensure_table_entry(table, index, pages)?;

        map_level(addr_bits - 9, table_ref(table, index), len, vaddr, flags, pages, map_block)?;
    }

    Ok(())
}

fn map_granuales<F>(addr_bits: usize, table: &mut [u64], index: &mut usize, len: &mut usize, vaddr: &mut VirtualAddress, flags: u64, pages: &mut PagePool, map_block: &F) -> Result<bool, KernelError>
where
    F: Fn(&mut PagePool, VirtualAddress, usize) -> Option<PhysicalAddress>
{
    let granuale_size = 1 << addr_bits;
    let block_flag = if addr_bits == 12 { TT3_DESCRIPTOR_BLOCK } else { TT2_DESCRIPTOR_BLOCK };

    while *len >= granuale_size {
        if descriptor_type(table, *index) != TT_DESCRIPTOR_EMPTY {
            return Err(KernelError::AddressAlreadyMapped);
        }

        if let Some(paddr) = map_block(pages, *vaddr, granuale_size) {
            table[*index] = (u64::from(paddr) & TT_BLOCK_MASK) | flags | block_flag;
        } else {
            return Ok(false);
        }

        *index += 1;
        *vaddr = vaddr.add(granuale_size);
        *len -= granuale_size;

        if *index >= table_entries() {
            // If we've reached the end of this table, then return to allow a higher level to increment its index
            break;
        }
    }

    Ok(true)
}

fn ensure_table_entry(table: &mut [u64], index: usize, pages: &mut PagePool) -> Result<(), KernelError> {
    let desc_type = descriptor_type(table, index);

    match desc_type {
        TT2_DESCRIPTOR_TABLE => {
            // Do nothing. Sub-table is already present
            Ok(())
        },

        TT_DESCRIPTOR_EMPTY => {
            let next_table = allocate_table(pages);
            table[index] = (u64::from(next_table) & TT_TABLE_MASK) | TT2_DESCRIPTOR_TABLE;
            Ok(())
        },
        TT2_DESCRIPTOR_BLOCK => {
            Err(KernelError::AddressAlreadyMapped)
        },
        _ => {
            Err(KernelError::CorruptTranslationTable)
        },
    }
}

fn unmap_level<F>(addr_bits: usize, table: &mut [u64], len: &mut usize, vaddr: &mut VirtualAddress, pages: &mut PagePool, unmap_block: &F) -> Result<(), KernelError>
where
    F: Fn(&mut PagePool, VirtualAddress, PhysicalAddress)
{
    let granuale_size = 1 << addr_bits;

    let mut index = table_index_from_vaddr(addr_bits, *vaddr);
    while *len > 0 && index <= table_entries() {
        if is_block(addr_bits, table, index) {
            unmap_granuales(addr_bits, table, &mut index, len, vaddr, pages, unmap_block)?;
        }

        if addr_bits != 12 && descriptor_type(table, index) == TT2_DESCRIPTOR_TABLE {
            let subtable = table_ref(table, index);
            unmap_level(addr_bits - 9, subtable, len, vaddr, pages, unmap_block)?;

            if table_is_empty(subtable) {
                pages.free_page(PhysicalAddress::from(subtable.as_ptr() as u64));
                table[index] = 0;
            }
        } else {
            *vaddr = vaddr.add(granuale_size);
            *len -= granuale_size;
            index += 1;
        }
    }

    Ok(())
}

fn unmap_granuales<F>(addr_bits: usize, table: &mut [u64], index: &mut usize, len: &mut usize, vaddr: &mut VirtualAddress, pages: &mut PagePool, unmap_block: &F) -> Result<(), KernelError>
where
    F: Fn(&mut PagePool, VirtualAddress, PhysicalAddress)
{
    let granuale_size = 1 << addr_bits;

    while *len >= granuale_size {
        if descriptor_type(table, *index) != TT_DESCRIPTOR_EMPTY {
            unmap_block(pages, *vaddr, block_ptr(table, *index));
            table[*index] = 0;
        }

        *index += 1;
        *vaddr = vaddr.add(granuale_size);
        *len -= granuale_size;

        if *index >= table_entries() {
            // If we've reached the end of this table, then return to allow a higher level to increment its index
            break;
        }
    }

    Ok(())
}

fn lookup_level(addr_bits: usize, table: &mut [u64], vaddr: VirtualAddress) -> Result<(&mut u64, usize), KernelError> {
    let granuale_size = 1 << addr_bits;

    let index = table_index_from_vaddr(addr_bits, vaddr);
    if is_block(addr_bits, table, index) {
        Ok((&mut table[index], granuale_size))
    } else if addr_bits == 12 {
        Err(KernelError::AddressUnmapped)
    } else {
        lookup_level(addr_bits - 9, table_ref(table, index), vaddr)
    }
}

fn allocate_table(pages: &mut PagePool) -> PhysicalAddress {
    pages.alloc_page_zeroed()
}



fn table_index_from_vaddr(bits: usize, vaddr: VirtualAddress) -> usize {
    ((usize::from(vaddr) >> bits) & 0x1ff) as usize
}

fn table_ref(table: &mut [u64], index: usize) -> &mut [u64] {
    table_as_slice(table_ptr(table, index))
}

fn table_as_slice(paddr: PhysicalAddress) -> &'static mut [u64] {
    unsafe {
        slice::from_raw_parts_mut(paddr.to_kernel_addr().as_mut(), table_entries())
    }
}

fn table_ptr(table: &mut [u64], index: usize) -> PhysicalAddress {
    PhysicalAddress::from(table[index] & TT_TABLE_MASK)
}

fn block_ptr(table: &mut [u64], index: usize) -> PhysicalAddress {
    PhysicalAddress::from(table[index] & TT_BLOCK_MASK)
}

fn descriptor_type(table: &mut [u64], index: usize) -> u64 {
    table[index] & TT_TYPE_MASK
}

fn is_block(addr_bits: usize, table: &mut [u64], index: usize) -> bool {
    let dtype = descriptor_type(table, index);
    if addr_bits == 12 {
        dtype == TT3_DESCRIPTOR_BLOCK
    } else {
        dtype == TT2_DESCRIPTOR_BLOCK
    }
}

fn table_is_empty(table: &mut [u64]) -> bool {
    for index in 0..table_entries() {
        if descriptor_type(table, index) != TT_DESCRIPTOR_EMPTY {
            return false;
        }
    }
    true
}

const fn attribute_index(index: u64) -> u64 {
    (index & 0x7) << 2
}

const fn memory_type_flags(mtype: MemoryType) -> u64 {
    match mtype {
        MemoryType::Unallocated => 0 | attribute_index(0),
        MemoryType::Existing => TT_ACCESS_FLAG | attribute_index(0),
        MemoryType::ExistingNoCache => TT_ACCESS_FLAG | attribute_index(1),
    }
}

const fn memory_permissions_flags(permissions: MemoryPermissions) -> u64 {
    match permissions {
        MemoryPermissions::ReadOnly => TT_READ_ONLY_FLAG | TT_NO_EXECUTE_FLAG,
        MemoryPermissions::ReadExecute => TT_READ_ONLY_FLAG,
        MemoryPermissions::ReadWrite => TT_READ_WRITE_FLAG | TT_NO_EXECUTE_FLAG,
        MemoryPermissions::ReadWriteExecute => TT_READ_WRITE_FLAG,
    }
}

