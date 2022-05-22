
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

const TT_COPY_ON_WRITE_FLAG: u64 = 1 << 58;

#[allow(dead_code)]
const TT_TYPE_MASK: u64 = 0b11;
const TT_TABLE_MASK: u64 = 0x0000_ffff_ffff_f000;
const TT_BLOCK_MASK: u64 = 0x0000_ffff_ffff_f000;
const TT_PERMISSIONS_MASK: u64 = 0b11 << 6;

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



pub struct TranslationTable(pub(super) u64);


#[inline(always)]
pub const fn page_size() -> usize {
    4096
}

#[inline(always)]
pub const fn table_entries() -> usize {
    page_size() / 8
}

pub fn get_page_slice(page: PhysicalAddress) -> &'static mut [u8] {
    unsafe {
        slice::from_raw_parts_mut(page.to_kernel_addr().as_mut(), page_size())
    }
}

impl TranslationTable {
    pub fn new_user_table(pages: &mut PagePool) -> Self {
        let tl0 = allocate_table(pages);
        Self(u64::from(tl0))
    }


    #[allow(dead_code)]
    pub fn map_existing_range(&mut self, access: MemoryPermissions, mut vaddr: VirtualAddress, mut paddr: PhysicalAddress, mut len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, len)?;

        let start_vaddr = vaddr;
        let flags = memory_type_flags(MemoryType::Existing) | memory_permissions_flags(access);
        map_level(TL0_ADDR_BITS, self.as_slice_mut(), &mut len, &mut vaddr, pages, &mut |_, current_vaddr, _| {
            let voffset = usize::from(current_vaddr) - usize::from(start_vaddr);
            Ok(Some((paddr.add(voffset), flags)))
        })
    }

    pub fn map_paged_range(&mut self, mtype: MemoryType, access: MemoryPermissions, mut vaddr: VirtualAddress, mut len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, len)?;

        let flags = memory_type_flags(mtype) | memory_permissions_flags(access);
        map_level(TL0_ADDR_BITS, self.as_slice_mut(), &mut len, &mut vaddr, pages, &mut |pages, _, len| {
            if len != page_size() {
                Ok(None) // Don't map granuales larger than a page
            } else if mtype == MemoryType::Allocated {
                Ok(Some((pages.alloc_page_zeroed(), flags)))
            } else {
                Ok(Some((PhysicalAddress::from(0), flags)))
            }
        })
    }

    pub fn copy_paged_range(&mut self, parent_table: &Self, access: MemoryPermissions, mut vaddr: VirtualAddress, mut len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, len)?;

        let flags = memory_type_flags(MemoryType::Unallocated) | memory_permissions_flags(access);

        map_level(TL0_ADDR_BITS, self.as_slice_mut(), &mut len, &mut vaddr, pages, &mut |pages, page_addr, len| {
            let (descriptor, _) = lookup_level(TL0_ADDR_BITS, parent_table.as_slice(), page_addr)?;
            if len != page_size() {
                Ok(None) // Don't map granuales larger than a page
            } else if *descriptor & TT_BLOCK_MASK == 0 {
                Ok(Some((PhysicalAddress::from(0), flags)))
            } else {
                Ok(Some((pages.ref_page(PhysicalAddress::from(*descriptor & TT_BLOCK_MASK)), TT_ACCESS_FLAG | flags)))
            }
        })
    }


    pub fn copy_pages_in_range(&mut self, parent_table: &Self, mut vaddr: VirtualAddress, mut len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, len)?;

        let flags = memory_type_flags(MemoryType::Unallocated) | memory_permissions_flags(MemoryPermissions::ReadWrite);

        map_level(TL0_ADDR_BITS, self.as_slice_mut(), &mut len, &mut vaddr, pages, &mut |pages, page_addr, len| {
            let (descriptor, _) = lookup_level(TL0_ADDR_BITS, parent_table.as_slice(), page_addr)?;
            if len != page_size() {
                Ok(None) // Don't map granuales larger than a page
            } else if *descriptor & TT_BLOCK_MASK == 0 {
                Ok(Some((PhysicalAddress::from(0), flags)))
            } else {
                let new_page = pages.alloc_page_zeroed();

                // Copy data into new page
                let page_buffer = get_page_slice(PhysicalAddress::from(*descriptor & TT_BLOCK_MASK));
                let new_page_buffer = get_page_slice(new_page);
                for i in 0..page_buffer.len() {
                    new_page_buffer[i] = page_buffer[i];
                }

                Ok(Some((PhysicalAddress::from(new_page), TT_ACCESS_FLAG | flags)))
            }
        })
    }

    pub fn unmap_range(&mut self, mut vaddr: VirtualAddress, mut len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, len)?;

        let free_pages_fn = |pages: &mut PagePool, _, paddr| {
            if usize::from(paddr) != 0 {
                pages.free_page(paddr)
            }
        };
        let mut visitor = UnmapRange::new(pages, free_pages_fn);
        visitor.visit_level(TL0_ADDR_BITS, self.as_slice_mut(), &mut len, &mut vaddr)
    }

    pub fn update_permissions(&mut self, perms: MemoryPermissions, mut vaddr: VirtualAddress, mut len: usize) -> Result<(), KernelError> {
        let mut visitor = ChangeBits::new(TT_PERMISSIONS_MASK | TT_COPY_ON_WRITE_FLAG, memory_permissions_flags(perms) | TT_COPY_ON_WRITE_FLAG);
        visitor.visit_level(TL0_ADDR_BITS, self.as_slice_mut(), &mut len, &mut vaddr)
    }


    pub fn update_addr(&mut self, vaddr: VirtualAddress, paddr: PhysicalAddress, expected_size: usize) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, expected_size)?;

        let (descriptor, granuale_size) = lookup_level_mut(TL0_ADDR_BITS, self.as_slice_mut(), vaddr)?;
        if granuale_size == expected_size {
            *descriptor &= !TT_BLOCK_MASK;
            *descriptor |= (u64::from(paddr) & TT_BLOCK_MASK) | TT_ACCESS_FLAG;
            Ok(())
        } else {
            Err(KernelError::UnexpectedGranualeSize)
        }
    }

    pub fn translate_addr(&self, vaddr: VirtualAddress) -> Result<PhysicalAddress, KernelError> {
        let (descriptor, granuale_size) = lookup_level(TL0_ADDR_BITS, self.as_slice(), vaddr.align_down(page_size()))?;
        Ok(PhysicalAddress::from(*descriptor & TT_BLOCK_MASK).add(vaddr.offset_from_align(granuale_size)))
    }

    pub(crate) fn get_ttbr(&self) -> u64 {
        self.0 as u64
    }

    fn as_slice(&self) -> &[u64] {
        table_as_slice(PhysicalAddress::from(self.0))
    }

    fn as_slice_mut(&mut self) -> &mut [u64] {
        table_as_slice_mut(PhysicalAddress::from(self.0))
    }
}

fn check_vaddr_and_usize(vaddr: VirtualAddress, len: usize) -> Result<(), KernelError> {
    if usize::from(vaddr) & (page_size() - 1) != 0 {
        Err(KernelError::AddressMisaligned)
    } else if len % page_size() != 0 {
        Err(KernelError::UnexpectedGranualeSize)
    } else {
        Ok(())
    }
}


fn map_level<F>(addr_bits: usize, table: &mut [u64], len: &mut usize, vaddr: &mut VirtualAddress, pages: &mut PagePool, map_block: &mut F) -> Result<(), KernelError>
where
    F: FnMut(&mut PagePool, VirtualAddress, usize) -> Result<Option<(PhysicalAddress, u64)>, KernelError>
{
    let granuale_size = 1 << addr_bits;

    while *len > 0 {
        let mut index = table_index_from_vaddr(addr_bits, *vaddr);
        if vaddr.offset_from_align(granuale_size) == 0 && *len >= granuale_size {
            let should_break = map_granuales(addr_bits, table, &mut index, len, vaddr, pages, map_block)?;
            if should_break {
                break;
            }
        }

        if addr_bits != 12 {
            ensure_table_entry(table, index, pages)?;

            map_level(addr_bits - 9, table_ref_mut(table, index), len, vaddr, pages, map_block)?;
        }
    }

    Ok(())
}

fn map_granuales<F>(addr_bits: usize, table: &mut [u64], index: &mut usize, len: &mut usize, vaddr: &mut VirtualAddress, pages: &mut PagePool, map_block: &mut F) -> Result<bool, KernelError>
where
    F: FnMut(&mut PagePool, VirtualAddress, usize) -> Result<Option<(PhysicalAddress, u64)>, KernelError>
{
    let granuale_size = 1 << addr_bits;
    let block_flag = if addr_bits == 12 { TT3_DESCRIPTOR_BLOCK } else { TT2_DESCRIPTOR_BLOCK };

    while *len >= granuale_size {
        if descriptor_type(table, *index) != TT_DESCRIPTOR_EMPTY {
            return Err(KernelError::AddressAlreadyMapped);
        }

        if let Some((paddr, flags)) = map_block(pages, *vaddr, granuale_size)? {
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

trait TableVisitor {
    fn visit_granuale(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError>;

    fn visit_table_before(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_table_after(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_level(&mut self, addr_bits: usize, table: &mut [u64], len: &mut usize, vaddr: &mut VirtualAddress) -> Result<(), KernelError> {
        walk_level(self, addr_bits, table, len, vaddr)
    }

    fn visit_all_granuales(&mut self, addr_bits: usize, table: &mut [u64], index: &mut usize, len: &mut usize, vaddr: &mut VirtualAddress) -> Result<(), KernelError> {
        walk_all_granuales(self, addr_bits, table, index, len, vaddr)
    }
}

fn walk_level<V>(visitor: &mut V, addr_bits: usize, table: &mut [u64], len: &mut usize, vaddr: &mut VirtualAddress) -> Result<(), KernelError>
where
    V: TableVisitor + ?Sized
{
    let granuale_size = 1 << addr_bits;

    let mut index = table_index_from_vaddr(addr_bits, *vaddr);
    while *len > 0 && index < table_entries() {
        if is_block(addr_bits, table, index) {
            visitor.visit_all_granuales(addr_bits, table, &mut index, len, vaddr)?;
        }

        if addr_bits != 12 && descriptor_type(table, index) == TT2_DESCRIPTOR_TABLE {
            visitor.visit_table_before(addr_bits, table, index, *vaddr)?;

            let subtable = table_ref_mut(table, index);
            visitor.visit_level(addr_bits - 9, subtable, len, vaddr)?;

            visitor.visit_table_after(addr_bits, table, index, *vaddr)?;
        } else {
            *vaddr = vaddr.add(granuale_size);
            *len -= granuale_size;
            index += 1;
        }
    }

    Ok(())
}

fn walk_all_granuales<V>(visitor: &mut V, addr_bits: usize, table: &mut [u64], index: &mut usize, len: &mut usize, vaddr: &mut VirtualAddress) -> Result<(), KernelError>
where
    V: TableVisitor + ?Sized
{
    let granuale_size = 1 << addr_bits;

    while *len >= granuale_size {
        if descriptor_type(table, *index) != TT_DESCRIPTOR_EMPTY {
            visitor.visit_granuale(addr_bits, table, *index, *vaddr)?;
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


/// Unmap all pages in the given address range
pub struct UnmapRange<'a, F> {
    pages: &'a mut PagePool,
    unmap_block: F,
}

impl<'a, F> UnmapRange<'a, F>
where
    F: Fn(&mut PagePool, VirtualAddress, PhysicalAddress)
{
    pub fn new(pages: &'a mut PagePool, unmap_block: F) -> Self {
        Self {
            pages,
            unmap_block,
        }
    }
}

impl<'a, F> TableVisitor for UnmapRange<'a, F>
where
    F: Fn(&mut PagePool, VirtualAddress, PhysicalAddress)
{
    fn visit_granuale(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        (self.unmap_block)(self.pages, vaddr, block_ptr(table, index));
        table[index] = 0;
        Ok(())
    }

    fn visit_table_after(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        let subtable = table_ref_mut(table, index);
        if table_is_empty(subtable) {
            self.pages.free_page(table_ptr(table, index));
            table[index] = 0;
        }
        Ok(())
    }
}


/// Modify the descriptor bits in a given address range
pub struct ChangeBits {
    mask: u64,
    bits: u64,
}

impl ChangeBits {
    pub fn new(mask: u64, bits: u64) -> Self {
        Self {
            mask,
            bits,
        }
    }
}

impl TableVisitor for ChangeBits {
    fn visit_granuale(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        table[index] &= !self.mask;
        table[index] |= self.bits;
        Ok(())
    }
}



fn lookup_level(addr_bits: usize, table: &[u64], vaddr: VirtualAddress) -> Result<(&u64, usize), KernelError> {
    let granuale_size = 1 << addr_bits;

    let index = table_index_from_vaddr(addr_bits, vaddr);
    if is_block(addr_bits, table, index) {
        Ok((&table[index], granuale_size))
    } else if addr_bits == 12 {
        Err(KernelError::AddressUnmapped)
    } else {
        lookup_level(addr_bits - 9, table_ref(table, index), vaddr)
    }
}

fn lookup_level_mut(addr_bits: usize, table: &mut [u64], vaddr: VirtualAddress) -> Result<(&mut u64, usize), KernelError> {
    let granuale_size = 1 << addr_bits;

    let index = table_index_from_vaddr(addr_bits, vaddr);
    if is_block(addr_bits, table, index) {
        Ok((&mut table[index], granuale_size))
    } else if addr_bits == 12 {
        Err(KernelError::AddressUnmapped)
    } else {
        lookup_level_mut(addr_bits - 9, table_ref_mut(table, index), vaddr)
    }
}

fn allocate_table(pages: &mut PagePool) -> PhysicalAddress {
    pages.alloc_page_zeroed()
}



fn table_index_from_vaddr(bits: usize, vaddr: VirtualAddress) -> usize {
    ((usize::from(vaddr) >> bits) & 0x1ff) as usize
}

fn table_ref(table: &[u64], index: usize) -> &[u64] {
    table_as_slice(table_ptr(table, index))
}

fn table_ref_mut(table: &mut [u64], index: usize) -> &mut [u64] {
    table_as_slice_mut(table_ptr(table, index))
}

fn table_as_slice(paddr: PhysicalAddress) -> &'static [u64] {
    unsafe {
        slice::from_raw_parts(paddr.to_kernel_addr().as_ptr(), table_entries())
    }
}

fn table_as_slice_mut(paddr: PhysicalAddress) -> &'static mut [u64] {
    unsafe {
        slice::from_raw_parts_mut(paddr.to_kernel_addr().as_mut(), table_entries())
    }
}

fn table_ptr(table: &[u64], index: usize) -> PhysicalAddress {
    PhysicalAddress::from(table[index] & TT_TABLE_MASK)
}

fn block_ptr(table: &[u64], index: usize) -> PhysicalAddress {
    PhysicalAddress::from(table[index] & TT_BLOCK_MASK)
}

fn descriptor_type(table: &[u64], index: usize) -> u64 {
    table[index] & TT_TYPE_MASK
}

fn is_block(addr_bits: usize, table: &[u64], index: usize) -> bool {
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
        MemoryType::Allocated => TT_ACCESS_FLAG | attribute_index(0),
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

