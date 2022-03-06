
use core::arch::asm;

#[allow(dead_code)]
pub unsafe fn get_current_el() -> u64 {
    let mut el: u64 = 0xffff;
    asm!(
        "mrs    {el}, CurrentEL",
        "lsr    {el}, {el}, 2",
        el = inout(reg) el,
    );
    el
}

