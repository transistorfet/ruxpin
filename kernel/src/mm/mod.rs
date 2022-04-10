 
pub mod pages;
pub mod segments;
pub mod kmalloc;
pub mod vmalloc;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum MemoryType {
    Unallocated,
    Existing,
    ExistingNoCache,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum MemoryPermissions {
    ReadOnly,
    ReadExecute,
    ReadWrite,
    ReadWriteExecute,
}

