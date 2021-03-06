
use ruxpin_kernel::irqs;
use ruxpin_kernel::notice;
use ruxpin_kernel::proc::scheduler;
use ruxpin_kernel::arch::KernelVirtualAddress;
use ruxpin_kernel::misc::deviceio::DeviceRegisters;

mod registers {
    pub const CONTROL: usize = 0x00;
    pub const COUNT_LOW: usize = 0x04;
    pub const COMPARE_1: usize = 0x10;
}

const SYS_TIMER: DeviceRegisters<u32> = DeviceRegisters::new(KernelVirtualAddress::new(0x3F00_3000));

pub struct SystemTimer;

impl SystemTimer {
    pub fn init(irq: usize) {
        notice!("timer: initializing generic arm timer to trigger context switch");

        irqs::register_irq(irq ,SystemTimer::handle_irq).unwrap();
        irqs::enable_irq(irq);

        unsafe {
            let value = SYS_TIMER.get(registers::COUNT_LOW) + 20000;
            SYS_TIMER.set(registers::COMPARE_1, value);
        }
    }

    pub fn reset() {
        unsafe {
            SYS_TIMER.set(registers::CONTROL, 1 << 1);
            let value = SYS_TIMER.get(registers::COUNT_LOW) + 20000;
            SYS_TIMER.set(registers::COMPARE_1, value);
        }
    }

    fn handle_irq() {
        SystemTimer::reset();
        scheduler::schedule();
    }
}

