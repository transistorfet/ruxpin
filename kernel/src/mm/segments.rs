
use alloc::boxed::Box;

use ruxpin_api::types::Seek;

use crate::fs::vfs;
use crate::fs::types::File;
use crate::errors::KernelError;
use crate::arch::types::VirtualAddress;


pub trait SegmentOperations: Sync + Send {
    // This assumes the page would be allocated automatically by the page fault handler, and this would just populate the data
    fn load_page_at(&self, segment: &Segment, vaddr: VirtualAddress, page: &mut [u8]) -> Result<(), KernelError>;
}

pub struct Segment {
    pub(super) start: VirtualAddress,
    pub(super) end: VirtualAddress,
    pub(super) ops: Box<dyn SegmentOperations>,
}

impl Segment {
    pub fn new(start: VirtualAddress, end: VirtualAddress, ops: Box<dyn SegmentOperations>) -> Self {
        Self {
            start,
            end,
            ops,
        }
    }

    pub fn match_range(&self, addr: VirtualAddress) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn new_memory(start: VirtualAddress, end: VirtualAddress) -> Self {
        let ops = Box::new(MemorySegment::new());
        Self::new(start, end, ops)
    }

    pub fn new_file_backed(file: File, file_offset: usize, file_size: usize, mem_offset: usize, start: VirtualAddress, end: VirtualAddress) -> Self {
        let ops = Box::new(FileBackedSegment::new(file, file_offset, file_size, mem_offset));
        Self::new(start, end, ops)
    }
}



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
        crate::printkln!("swapping {:?} for segment from {:?}", vaddr, segment.start);

        let segment_offset = usize::from(vaddr) - usize::from(segment.start);
        let file_offset = self.file_offset + segment_offset.saturating_sub(self.mem_offset);
        vfs::seek(self.file.clone(), file_offset, Seek::FromStart)?;

        let buffer_start = if segment_offset < page.len() { self.mem_offset } else { 0 };
        let buffer_len = if file_offset + self.file_size < page.len() - buffer_start { file_offset + self.file_size } else { page.len() - buffer_start };
        vfs::read(self.file.clone(), &mut page[buffer_start..(buffer_start + buffer_len)])?;

        crate::printkln!("file offset: {:x}  segment offset: {:x}  buffer_start: {:x}  buffer_len: {:x}", file_offset, segment_offset, buffer_start, buffer_len);
        Ok(())
    }
}

