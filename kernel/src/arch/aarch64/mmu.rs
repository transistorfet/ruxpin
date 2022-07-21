
use core::mem;
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
    page_size() / mem::size_of::<u64>()
}

pub fn get_page_slice(page: PhysicalAddress) -> &'static mut [u8] {
    unsafe {
        slice::from_raw_parts_mut(page.to_kernel_addr().as_mut(), page_size())
    }
}

impl TranslationTable {
    pub fn initial_kernel_table() -> Self {
        use core::arch::asm;

        let mut ttbr;
        unsafe {
            asm!(
                "mrs  {}, TTBR1_EL1",
                out(reg) ttbr,
            );
        }
        Self(ttbr)
    }

    pub fn new_table(pages: &mut PagePool) -> Self {
        let tl0 = allocate_table(pages);
        Self(u64::from(tl0))
    }


    pub fn map_existing_range(&mut self, access: MemoryPermissions, start: VirtualAddress, paddr: PhysicalAddress, len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(start, len)?;

        let end = start.add(len);
        let mut mapper = MapRange::new(pages, MemoryType::Existing, access, |_, current_vaddr, _| {
            let voffset = usize::from(current_vaddr) - usize::from(start);
            Ok(Some(paddr.add(voffset)))
        });
        mapper.visit_table(TL0_ADDR_BITS, self.as_slice_mut(), start, end)
    }

    pub fn map_paged_range(&mut self, mtype: MemoryType, access: MemoryPermissions, start: VirtualAddress, len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(start, len)?;

        let end = start.add(len);

        let mut mapper = MapRange::new(pages, mtype, access, |pages, _, granuale_size| {
            if granuale_size != page_size() {
                Ok(None) // Don't map granuales larger than a page
            } else if mtype == MemoryType::Allocated {
                Ok(Some(pages.alloc_page_zeroed()))
            } else {
                Ok(Some(PhysicalAddress::from(0)))
            }
        });
        mapper.visit_table(TL0_ADDR_BITS, self.as_slice_mut(), start, end)
    }

    pub fn duplicate_paged_range(&mut self, parent_table: &mut Self, access: MemoryPermissions, start: VirtualAddress, len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(start, len)?;

        let end = start.add(len);
        let flags = memory_type_flags(MemoryType::Unallocated) | memory_permissions_flags(access);

        let mut mapper = CopyRange::new(pages, |pages, _, parent_descriptor, granuale_size| {
            if granuale_size != page_size() {
                Err(KernelError::UnexpectedGranualeSize)
            } else if *parent_descriptor & TT_BLOCK_MASK == 0 {
                Ok(Some((PhysicalAddress::from(0), flags)))
            } else {
                Ok(Some((pages.ref_page(PhysicalAddress::from(*parent_descriptor & TT_BLOCK_MASK)), TT_ACCESS_FLAG | flags)))
            }
        });
        mapper.visit_table(TL0_ADDR_BITS, parent_table.as_slice_mut(), self.as_slice_mut(), start, end)
    }

    pub fn remap_range_copy_on_write(&mut self, parent_table: &mut Self, start: VirtualAddress, len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(start, len)?;

        let end = start.add(len);
        let flags = TT_COPY_ON_WRITE_FLAG | TT_READ_ONLY_FLAG | attribute_index(0);

        let mut mapper = CopyRange::new(pages, |pages, _, parent_descriptor, granuale_size| {
            let page = *parent_descriptor & TT_BLOCK_MASK;

            if granuale_size != page_size() {
                Err(KernelError::UnexpectedGranualeSize)
            } else if page == 0 {
                Ok(Some((PhysicalAddress::from(0), flags)))
            } else {
                *parent_descriptor = (*parent_descriptor & !TT_PERMISSIONS_MASK) | flags;
                Ok(Some((pages.ref_page(PhysicalAddress::from(page)), TT_ACCESS_FLAG | flags)))
            }
        });
        mapper.visit_table(TL0_ADDR_BITS, parent_table.as_slice_mut(), self.as_slice_mut(), start, end)
    }

    pub fn unmap_range(&mut self, start: VirtualAddress, len: usize, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(start, len)?;

        let end = start.add(len);
        let free_pages_fn = |pages: &mut PagePool, _, paddr| {
            pages.free_page(paddr)
        };
        let mut visitor = UnmapRange::new(pages, free_pages_fn);
        visitor.visit_table(TL0_ADDR_BITS, self.as_slice_mut(), start, end)
    }

    pub fn set_page_copy_on_write(&mut self, vaddr: VirtualAddress) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, page_size())?;

        let (descriptor, granuale_size) = lookup_level_mut(TL0_ADDR_BITS, self.as_slice_mut(), vaddr, None)?;
        if granuale_size == page_size() {
            *descriptor = (*descriptor & !TT_PERMISSIONS_MASK) | TT_COPY_ON_WRITE_FLAG | TT_READ_ONLY_FLAG;
            Ok(())
        } else {
            Err(KernelError::UnexpectedGranualeSize)
        }
    }

    pub fn reset_page_copy_on_write(&mut self, vaddr: VirtualAddress) -> Result<(PhysicalAddress, bool), KernelError> {
        check_vaddr_and_usize(vaddr, page_size())?;

        let (descriptor, granuale_size) = lookup_level_mut(TL0_ADDR_BITS, self.as_slice_mut(), vaddr, None)?;
        if granuale_size == page_size() {
            let previous_cow = (*descriptor & TT_COPY_ON_WRITE_FLAG) == TT_COPY_ON_WRITE_FLAG;
            *descriptor = (*descriptor & !(TT_PERMISSIONS_MASK | TT_COPY_ON_WRITE_FLAG)) | TT_READ_WRITE_FLAG;
            Ok((PhysicalAddress::from(*descriptor & TT_BLOCK_MASK), previous_cow))
        } else {
            Err(KernelError::UnexpectedGranualeSize)
        }
    }

    pub fn update_page_addr(&mut self, vaddr: VirtualAddress, paddr: PhysicalAddress, pages: &mut PagePool) -> Result<(), KernelError> {
        check_vaddr_and_usize(vaddr, page_size())?;

        let (descriptor, granuale_size) = lookup_level_mut(TL0_ADDR_BITS, self.as_slice_mut(), vaddr, Some(pages))?;
        if granuale_size == page_size() {
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
        table_as_slice(PhysicalAddress::from(self.0)).unwrap()
    }

    fn as_slice_mut(&mut self) -> &mut [u64] {
        table_as_slice_mut(PhysicalAddress::from(self.0)).unwrap()
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


/// Generic Visitor for a TranslationTable, which traverses each entry in the tree of tables
trait TableVisitor {
    fn visit_granuale(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError>;

    fn visit_table_before(&mut self, _addr_bits: usize, _table: &mut [u64], _index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_table_after(&mut self, _addr_bits: usize, _table: &mut [u64], _index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_table(&mut self, addr_bits: usize, table: &mut [u64], start: VirtualAddress, end: VirtualAddress) -> Result<(), KernelError> {
        walk_table(self, addr_bits, table, start, end)
    }

    fn visit_empty(&mut self, _addr_bits: usize, _table: &mut [u64], _index: usize, _vaddr: VirtualAddress, _end: VirtualAddress) -> Result<(), KernelError> {
        Err(KernelError::AddressUnmapped)
    }
}

fn walk_table<V>(visitor: &mut V, addr_bits: usize, table: &mut [u64], mut vaddr: VirtualAddress, end: VirtualAddress) -> Result<(), KernelError>
where
    V: TableVisitor + ?Sized
{
    let granuale_size = 1 << addr_bits;
    let mut index = table_index_from_vaddr(addr_bits, vaddr);
    while vaddr < end && index < table_entries() {
        if is_block(addr_bits, table, index) {
            visitor.visit_granuale(addr_bits, table, index, vaddr)?;
        } else if is_table(addr_bits, table, index) {
            visitor.visit_table_before(addr_bits, table, index, vaddr)?;

            if let Ok(subtable) = table_ref_mut(table, index) {
                visitor.visit_table(addr_bits - 9, subtable, vaddr, end)?;
            }

            visitor.visit_table_after(addr_bits, table, index, vaddr)?;
        } else if is_empty(table, index) {
            visitor.visit_empty(addr_bits, table, index, vaddr, end)?;
        } else {
            return Err(KernelError::CorruptTranslationTable);
        }

        vaddr = vaddr.add(granuale_size);
        index += 1;
    }
    Ok(())
}


/// Map 4K pages in a given range using a callback to allocate them
struct MapRange<'a, F> {
    pages: &'a mut PagePool,
    mtype: MemoryType,
    flags: u64,
    map_block: F,
}

impl<'a, F> MapRange<'a, F>
where
    F: FnMut(&mut PagePool, VirtualAddress, usize) -> Result<Option<PhysicalAddress>, KernelError>,
{
    fn new(pages: &'a mut PagePool, mtype: MemoryType, access: MemoryPermissions, map_block: F) -> Self {
        let flags = memory_type_flags(mtype) | memory_permissions_flags(access);

        Self {
            pages,
            mtype,
            flags,
            map_block,
        }
    }
}

impl<'a, F> TableVisitor for MapRange<'a, F>
where
    F: FnMut(&mut PagePool, VirtualAddress, usize) -> Result<Option<PhysicalAddress>, KernelError>,
{
    fn visit_granuale(&mut self, _addr_bits: usize, _table: &mut [u64], _index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        Err(KernelError::AddressAlreadyMapped)
    }

    fn visit_empty(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress, end: VirtualAddress) -> Result<(), KernelError> {
        let granuale_size = 1 << addr_bits;

        if addr_bits != 12 {
            if self.mtype != MemoryType::Unallocated || vaddr.add(granuale_size) > end {
                ensure_table_entry(table, index, self.pages)?;

                let subtable = table_ref_mut(table, index)?;
                walk_table(self, addr_bits - 9, subtable, vaddr, end)?;
            } else {
                table[index] = self.flags | TT2_DESCRIPTOR_TABLE;
            }
        } else {
            if let Some(paddr) = (self.map_block)(self.pages, vaddr, granuale_size)? {
                table[index] = (u64::from(paddr) & TT_BLOCK_MASK) | self.flags | block_type(granuale_size);
            }
        }
        Ok(())
    }
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


/// Unmap all pages in the given address range
struct UnmapRange<'a, F> {
    pages: &'a mut PagePool,
    unmap_block: F,
}

impl<'a, F> UnmapRange<'a, F>
where
    F: Fn(&mut PagePool, VirtualAddress, PhysicalAddress)
{
    fn new(pages: &'a mut PagePool, unmap_block: F) -> Self {
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
    fn visit_granuale(&mut self, _addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        let paddr = block_ptr(table, index);
        if usize::from(paddr) != 0 {
            (self.unmap_block)(self.pages, vaddr, paddr);
        }
        table[index] = 0;
        Ok(())
    }

    fn visit_table_after(&mut self, _addr_bits: usize, table: &mut [u64], index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        if let Ok(subtable) = table_ref_mut(table, index) {
            if table_is_empty(subtable) {
                self.pages.free_page(table_ptr(table, index));
                table[index] = 0;
            }
        }
        Ok(())
    }
}


/// Visitor to print the contents of the table for debugging and inspection
struct PrintTable {}

impl PrintTable {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }
}

impl TableVisitor for PrintTable {
    fn visit_granuale(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        crate::info!("granuale of {} at index {} ({:?}, {:?})", addr_bits, index, vaddr, table_ptr(table, index));
        Ok(())
    }

    fn visit_table_before(&mut self, addr_bits: usize, table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        crate::info!("table of {} at index {} ({:?}, {:?})", addr_bits, index, vaddr, table_ptr(table, index));
        Ok(())
    }

    fn visit_table_after(&mut self, _addr_bits: usize, _table: &mut [u64], _index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_empty(&mut self, addr_bits: usize, _table: &mut [u64], index: usize, vaddr: VirtualAddress, _end: VirtualAddress) -> Result<(), KernelError> {
        crate::info!("empty entry of {} at index {} ({:?})", addr_bits, index, vaddr);
        Ok(())
    }
}


/// Visitor for traversing two TranslationTable over the same address range
trait TwoTableVisitor {
    fn visit_granuale(&mut self, addr_bits: usize, _parent_table: &mut [u64], _child_table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError>;

    fn visit_table_before(&mut self, _addr_bits: usize, _parent_table: &mut [u64], _child_table: &mut [u64], _index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_table_after(&mut self, _addr_bits: usize, _parent_table: &mut [u64], _child_table: &mut [u64], _index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        Ok(())
    }

    fn visit_table(&mut self, addr_bits: usize, parent_table: &mut [u64], child_table: &mut [u64], start: VirtualAddress, end: VirtualAddress) -> Result<(), KernelError> {
        walk_two_tables(self, addr_bits, parent_table, child_table, start, end)
    }

    fn visit_empty(&mut self, _addr_bits: usize, _parent_table: &mut [u64], _child_table: &mut [u64], _index: usize, _vaddr: VirtualAddress, _end: VirtualAddress) -> Result<(), KernelError> {
        Err(KernelError::AddressUnmapped)
    }
}

fn walk_two_tables<V>(visitor: &mut V, addr_bits: usize, parent_table: &mut [u64], child_table: &mut [u64], mut vaddr: VirtualAddress, end: VirtualAddress) -> Result<(), KernelError>
where
    V: TwoTableVisitor + ?Sized
{
    let granuale_size = 1 << addr_bits;
    let mut index = table_index_from_vaddr(addr_bits, vaddr);
    while vaddr < end && index < table_entries() {
        if is_block(addr_bits, parent_table, index) {
            visitor.visit_granuale(addr_bits, parent_table, child_table, index, vaddr)?;
        } else if is_table(addr_bits, parent_table, index) {
            visitor.visit_table_before(addr_bits, parent_table, child_table, index, vaddr)?;

            if let Ok(parent_subtable) = table_ref_mut(parent_table, index) {
                if let Ok(child_subtable) = table_ref_mut(child_table, index) {
                    visitor.visit_table(addr_bits - 9, parent_subtable, child_subtable, vaddr, end)?;
                }
            }

            visitor.visit_table_after(addr_bits, parent_table, child_table, index, vaddr)?;
        } else if is_empty(parent_table, index) {
            visitor.visit_empty(addr_bits, parent_table, child_table, index, vaddr, end)?;
        } else {
            return Err(KernelError::CorruptTranslationTable);
        }

        vaddr = vaddr.add(granuale_size);
        index += 1;
    }
    Ok(())
}

/// Copy the mapping of 4K pages in a given range
struct CopyRange<'a, F> {
    pages: &'a mut PagePool,
    map_block: F,
}

impl<'a, F> CopyRange<'a, F>
where
    F: FnMut(&mut PagePool, VirtualAddress, &mut u64, usize) -> Result<Option<(PhysicalAddress, u64)>, KernelError>,
{
    fn new(pages: &'a mut PagePool, map_block: F) -> Self {
        Self {
            pages,
            map_block,
        }
    }
}

impl<'a, F> TwoTableVisitor for CopyRange<'a, F>
where
    F: FnMut(&mut PagePool, VirtualAddress, &mut u64, usize) -> Result<Option<(PhysicalAddress, u64)>, KernelError>,
{
    fn visit_granuale(&mut self, addr_bits: usize, parent_table: &mut [u64], child_table: &mut [u64], index: usize, vaddr: VirtualAddress) -> Result<(), KernelError> {
        let granuale_size = 1 << addr_bits;
        if let Some((paddr, flags)) = (self.map_block)(self.pages, vaddr, &mut parent_table[index], granuale_size)? {
            child_table[index] = (u64::from(paddr) & TT_BLOCK_MASK) | flags | block_type(granuale_size);
        }
        Ok(())
    }

    fn visit_table_before(&mut self, _addr_bits: usize, parent_table: &mut [u64], child_table: &mut [u64], index: usize, _vaddr: VirtualAddress) -> Result<(), KernelError> {
        if table_ref_mut(parent_table, index).is_ok() {
            ensure_table_entry(child_table, index, self.pages)?;
        } else {
            child_table[index] = (parent_table[index] & (TT_PERMISSIONS_MASK | TT_COPY_ON_WRITE_FLAG)) | TT2_DESCRIPTOR_TABLE;
        }
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
        lookup_level(addr_bits - 9, table_ref(table, index)?, vaddr)
    }
}

fn lookup_level_mut<'a>(addr_bits: usize, table: &'a mut [u64], vaddr: VirtualAddress, mut pages: Option<&mut PagePool>) -> Result<(&'a mut u64, usize), KernelError> {
    let granuale_size = 1 << addr_bits;

    let index = table_index_from_vaddr(addr_bits, vaddr);
    if is_block(addr_bits, table, index) {
        Ok((&mut table[index], granuale_size))
    } else if addr_bits == 12 {
        Err(KernelError::AddressUnmapped)
    } else {
        if table_ref_mut(table, index).is_err() {
            if let Some(pages) = &mut pages {
                let next_table = allocate_table(pages);

                table[index] |= u64::from(next_table) & TT_TABLE_MASK;

                let parent_descriptor = table[index];
                initialize_table(granuale_size, table_ref_mut(table, index)?, parent_descriptor)?;
            } else {
                return Err(KernelError::AddressUnmapped);
            }
        }
        lookup_level_mut(addr_bits - 9, table_ref_mut(table, index)?, vaddr, pages)
    }
}

fn initialize_table(granuale_size: usize, table: &mut [u64], parent_descriptor: u64) -> Result<(), KernelError> {
    let dtype = if granuale_size == page_size() {
        TT3_DESCRIPTOR_BLOCK
    } else {
        TT2_DESCRIPTOR_TABLE
    };

    let flags = (parent_descriptor & (TT_PERMISSIONS_MASK | TT_COPY_ON_WRITE_FLAG)) | dtype;

    for index in 0..table_entries() {
        table[index] = flags;
    }

    Ok(())
}



fn allocate_table(pages: &mut PagePool) -> PhysicalAddress {
    pages.alloc_page_zeroed()
}

fn table_index_from_vaddr(bits: usize, vaddr: VirtualAddress) -> usize {
    ((usize::from(vaddr) >> bits) & 0x1ff) as usize
}

fn table_ref(table: &[u64], index: usize) -> Result<&[u64], KernelError> {
    table_as_slice(table_ptr(table, index))
}

fn table_ref_mut(table: &mut [u64], index: usize) -> Result<&mut [u64], KernelError> {
    table_as_slice_mut(table_ptr(table, index))
}

fn table_as_slice(paddr: PhysicalAddress) -> Result<&'static [u64], KernelError> {
    if u64::from(paddr) == 0 {
        Err(KernelError::CorruptTranslationTable)
    } else {
        unsafe {
            Ok(slice::from_raw_parts(paddr.to_kernel_addr().as_ptr(), table_entries()))
        }
    }
}

fn table_as_slice_mut(paddr: PhysicalAddress) -> Result<&'static mut [u64], KernelError> {
    if u64::from(paddr) == 0 {
        Err(KernelError::CorruptTranslationTable)
    } else {
        unsafe {
            Ok(slice::from_raw_parts_mut(paddr.to_kernel_addr().as_mut() as *mut u64, table_entries()))
        }
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

fn is_table(addr_bits: usize, table: &[u64], index: usize) -> bool {
    addr_bits != 12 && descriptor_type(table, index) == TT2_DESCRIPTOR_TABLE
}

fn is_empty(table: &[u64], index: usize) -> bool {
    descriptor_type(table, index) == 0
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

const fn block_type(granuale_size: usize) -> u64 {
    if granuale_size == page_size() {
        TT3_DESCRIPTOR_BLOCK
    } else {
        TT2_DESCRIPTOR_BLOCK
    }
}

