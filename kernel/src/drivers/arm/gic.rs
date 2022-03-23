

use core::ptr;

const GIC_BASE: u64 = 0x3F00_B200;

//const GIC_IRQ_PENDING_BASIC: *mut u32 = (GIC_BASE + 0x00) as *mut u32;
const GIC_IRQ_ENABLE1: *mut u32 = (GIC_BASE + 0x10) as *mut u32;
//const GIC_IRQ_DISABLE1: *mut u32 = (GIC_BASE + 0x1C) as *mut u32;


pub struct GenericInterruptController;

impl GenericInterruptController {
    pub fn init() {
    /*
        unsafe {
            asm!(
                "mrs    {tmp}, CNTFRQ_EL0",
                "msr    CNTP_TVAL_EL0, {tmp}",
                "mov    {tmp}, #1",
                "msr    CNTP_CTL_EL0, {tmp}",
                tmp = out(reg) _
            );
        }
    */


        GenericInterruptController::enable_interrupt();
    }

    pub fn enable_interrupt() {
        unsafe {
            ptr::write_volatile(GIC_IRQ_ENABLE1, 1 << 1);
            //printkln!("GIC: {:x}", ptr::read_volatile(GIC_IRQ_ENABLE1));
        }
    }
}

