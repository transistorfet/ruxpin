
use crate::arch::types::KernelVirtualAddress;
use crate::misc::deviceio::DeviceRegisters;


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








const GIC: DeviceRegisters<u32> = DeviceRegisters::new(KernelVirtualAddress::new(0x3F00_B200));

mod registers {
    //pub const IRQ_PENDING_BASIC: usize = 0x00;
    pub const IRQ_ENABLE1: usize = 0x10;
    pub const IRQ_ENABLE2: usize = 0x14;
    //pub const IRQ_DISABLE1: usize = 0x1C;
}


pub struct GenericInterruptController;

impl GenericInterruptController {
    pub fn init() {
        GenericInterruptController::enable_irq(1);
    }

    pub fn enable_irq(irq: usize) {
        unsafe {
            if irq < 32 {
                GIC.set(registers::IRQ_ENABLE1, 1 << irq);
            } else {
                GIC.set(registers::IRQ_ENABLE2, 1 << (irq - 32));
            }
        }
    }
}

