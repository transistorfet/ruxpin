
pub mod console;

core::arch::global_asm!(include_str!("start.s"));
core::arch::global_asm!(include_str!("exceptions.s"));

