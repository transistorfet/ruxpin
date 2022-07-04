 
pub mod pages;
pub mod pagecache;
pub mod segments;
pub mod kmalloc;
pub mod vmalloc;

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

