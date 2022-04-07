
use crate::arch::exceptions::register_irq;
use crate::arch::types::KernelVirtualAddress;
use crate::misc::deviceio::DeviceRegisters;

mod registers {
    pub const CONTROL: usize = 0x00;
    pub const COUNT_LOW: usize = 0x04;
    pub const COMPARE_1: usize = 0x10;
}

const SYS_TIMER: DeviceRegisters<u32> = DeviceRegisters::new(KernelVirtualAddress::new(0x3F00_3000));

pub struct SystemTimer;

impl SystemTimer {
    pub fn init() {
        register_irq(SystemTimer::handle_irq);

        unsafe {
            let value = SYS_TIMER.get(registers::COUNT_LOW) + 200000;
            SYS_TIMER.set(registers::COMPARE_1, value);
        }
    }

    pub fn reset() {
        unsafe {
            SYS_TIMER.set(registers::CONTROL, 1 << 1);
            let value = SYS_TIMER.get(registers::COUNT_LOW) + 200000;
            SYS_TIMER.set(registers::COMPARE_1, value);
        }
    }

    fn handle_irq() {
        SystemTimer::reset();
    }
}

