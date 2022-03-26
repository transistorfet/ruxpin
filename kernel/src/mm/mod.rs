 
pub mod pages;
pub mod kmalloc;
pub mod vmalloc;

#[no_mangle]
pub static __KERNEL_VIRTUAL_BASE_ADDR: u64 = 0xffff_0000_0000_0000;

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

