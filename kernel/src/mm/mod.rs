 
pub mod kmalloc;
pub mod vmalloc;

#[no_mangle]
pub static __KERNEL_VIRTUAL_BASE_ADDR: u64 = 0xffff_0000_0000_0000;

