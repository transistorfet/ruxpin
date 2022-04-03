

use core::ptr;


/*
- need a way to abstract the interrupt handler so that aarch64 code can use a different interrupt controller based on the specific board
*/

/*
struct IrqIter {

}

impl Iterator for IrqIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
*/

trait InterruptController {
    fn enable_irq(irq: usize);
    fn disable_irq(irq: usize);
    //fn pending_irqs() -> impl Iterator<Item=usize>;
}








const GIC_BASE: u64 = 0xFFFF_0000_3F00_B200;

//const GIC_IRQ_PENDING_BASIC: *mut u32 = (GIC_BASE + 0x00) as *mut u32;
const GIC_IRQ_ENABLE1: *mut u32 = (GIC_BASE + 0x10) as *mut u32;
const GIC_IRQ_ENABLE2: *mut u32 = (GIC_BASE + 0x14) as *mut u32;
//const GIC_IRQ_DISABLE1: *mut u32 = (GIC_BASE + 0x1C) as *mut u32;


pub struct GenericInterruptController;

impl GenericInterruptController {
    pub fn init() {
        GenericInterruptController::enable_irq(1);
    }

    pub fn enable_irq(irq: usize) {
        unsafe {
            if irq < 32 {
                ptr::write_volatile(GIC_IRQ_ENABLE1, 1 << irq);
            } else {
                ptr::write_volatile(GIC_IRQ_ENABLE2, 1 << (irq - 32));
            }
        }
    }
}

