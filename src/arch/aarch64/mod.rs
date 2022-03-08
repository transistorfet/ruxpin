
pub mod registers;

core::arch::global_asm!(include_str!("start.s"));
core::arch::global_asm!(include_str!("exceptions.s"));

extern {
    pub fn create_context(sp: *mut u8, entry: *mut u8) -> *mut u8;
    pub fn start_multitasking();
}

