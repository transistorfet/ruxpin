
use alloc::sync::Arc;
use alloc::boxed::Box;

use ruxpin_types::Seek;

use crate::fs::vfs;
use crate::arch::mmu;
use crate::misc::align_up;
use crate::sync::Spinlock;
use crate::fs::types::File;
use crate::mm::MemoryPermissions;
use crate::errors::KernelError;
use crate::arch::types::VirtualAddress;


#[derive(Copy, Clone, PartialEq)]
pub enum SegmentType {
    Text,
    Data,
    Stack,
}

pub trait SegmentOperations: Sync + Send {
    // This assumes the page would be allocated automatically by the page fault handler, and this would just populate the data
    fn load_page_at(&self, segment: &Segment, vaddr: VirtualAddress, page: &mut [u8]) -> Result<(), KernelError>;
}

pub struct Segment {
    pub(super) permissions: MemoryPermissions,
    pub(super) start: VirtualAddress,
    pub(super) end: VirtualAddress,
    ops: Box<dyn SegmentOperations>,
}

pub type ArcSegment = Arc<Spinlock<Segment>>;

impl Segment {
    pub fn new(permissions: MemoryPermissions, start: VirtualAddress, end: VirtualAddress, ops: Box<dyn SegmentOperations>) -> Self {
        Self {
            permissions,
            start,
            end,
            ops,
        }
    }

    pub fn new_memory(permissions: MemoryPermissions, start: VirtualAddress, end: VirtualAddress) -> Self {
        let ops = Box::new(MemorySegment::new());
        Self::new(permissions, start, end, ops)
    }

    pub fn new_file_backed(file: File, file_offset: usize, file_size: usize, permissions: MemoryPermissions, mem_offset: usize, start: VirtualAddress, end: VirtualAddress) -> Self {
        let ops = Box::new(FileBackedSegment::new(file, file_offset, file_size, mem_offset));
        Self::new(permissions, start, end, ops)
    }

    pub fn page_aligned_len(&self) -> usize {
        align_up(usize::from(self.end) - usize::from(self.start), mmu::page_size())
    }

    pub fn match_range(&self, addr: VirtualAddress) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn load_page_at(&self, segment: &Segment, vaddr: VirtualAddress, page: &mut [u8]) -> Result<(), KernelError> {
        self.ops.load_page_at(segment, vaddr, page)
    }
}



#[derive(Clone)]
pub struct MemorySegment {}

impl MemorySegment {
    pub fn new() -> Self {
        Self {}
    }
}

impl SegmentOperations for MemorySegment {
    fn load_page_at(&self, _segment: &Segment, _vaddr: VirtualAddress, _page: &mut [u8]) -> Result<(), KernelError> {
        Ok(())
    }
}


#[derive(Clone)]
pub struct FileBackedSegment {
    file: File,
    file_offset: usize,
    file_size: usize,
    mem_offset: usize,
}

impl FileBackedSegment {
    pub fn new(file: File, file_offset: usize, file_size: usize, mem_offset: usize) -> Self {
        Self {
            file,
            file_offset,
            file_size,
            mem_offset,
        }
    }
}

impl SegmentOperations for FileBackedSegment {
    fn load_page_at(&self, segment: &Segment, vaddr: VirtualAddress, page: &mut [u8]) -> Result<(), KernelError> {
        //crate::debug!("swapping {:?} for segment from {:?}", vaddr, segment.start);

        let segment_offset = usize::from(vaddr) - usize::from(segment.start);
        let file_offset = self.file_offset + segment_offset.saturating_sub(self.mem_offset);
        vfs::seek(self.file.clone(), file_offset, Seek::FromStart)?;

        let buffer_start = if segment_offset < page.len() { self.mem_offset } else { 0 };
        let buffer_len = if file_offset + self.file_size < page.len() - buffer_start { file_offset + self.file_size } else { page.len() - buffer_start };
        vfs::read(self.file.clone(), &mut page[buffer_start..(buffer_start + buffer_len)])?;

        //crate::debug!("file offset: {:x}  segment offset: {:x}  buffer_start: {:x}  buffer_len: {:x}", file_offset, segment_offset, buffer_start, buffer_len);
        Ok(())
    }
}

