
use core::ptr;
use core::arch::asm;

use crate::printkln;
use crate::mm::__KERNEL_VIRTUAL_BASE_ADDR;

//const GIC_BASE: u64 = 0xffff_0000_0000_0000 + 0x7E00_B000;
//const GIC_BASE: u64 = 0x3F00_B000;
const GIC_BASE: u64 = 0x3F00_B200;

const GIC_IRQ_PENDING_BASIC: *mut u32 = (GIC_BASE + 0x00) as *mut u32;
const GIC_IRQ_ENABLE1: *mut u32 = (GIC_BASE + 0x10) as *mut u32;
const GIC_IRQ_DISABLE1: *mut u32 = (GIC_BASE + 0x1C) as *mut u32;


pub fn init_gic() {
    unsafe {
/*
        asm!(
            "mrs    {tmp}, CNTFRQ_EL0",
            "msr    CNTP_TVAL_EL0, {tmp}",
            "mov    {tmp}, #1",
            "msr    CNTP_CTL_EL0, {tmp}",
            tmp = out(reg) _
        );
*/

    }

    timer_init();
    gic_enable_interrupt();
}

pub fn gic_enable_interrupt() {
    unsafe {
        ptr::write_volatile(GIC_IRQ_ENABLE1, 1 << 1);
        //printkln!("GIC: {:x}", ptr::read_volatile(GIC_IRQ_ENABLE1));
    }
}

pub fn timer_init() {
    unsafe {
        let value = ptr::read_volatile(0x3F003004 as *mut u32) + 200000;
        ptr::write_volatile(0x3F003010 as *mut u32, value);
    }
}

pub fn timer_reset() {
    unsafe {
        core::ptr::write_volatile(0x3F00_3000 as *mut u32, 1 << 1);
        let value = core::ptr::read_volatile(0x3F003004 as *mut u32) + 200000;
        core::ptr::write_volatile(0x3F003010 as *mut u32, value);
    }
}

