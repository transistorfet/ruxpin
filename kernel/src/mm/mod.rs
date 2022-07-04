 
pub mod pages;
pub mod pagecache;
pub mod kmalloc;
pub mod vmalloc;

mod segments;

pub use self::segments::SegmentType;
pub use self::vmalloc::{VirtualAddressSpace, SharableVirtualAddressSpace};


#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MemoryType {
    Unallocated,
    Allocated,
    Existing,
    ExistingNoCache,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MemoryPermissions {
    ReadOnly,
    ReadExecute,
    ReadWrite,
    ReadWriteExecute,
}

