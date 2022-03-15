
use core::ptr;

use crate::arch::exceptions::register_irq;

const SYS_TIMER_BASE: u64 = 0x3F00_3000;

const SYS_TIMER_CONTROL: *mut u32 = (SYS_TIMER_BASE + 0x00) as *mut u32;
const SYS_TIMER_COUNT_LOW: *mut u32 = (SYS_TIMER_BASE + 0x04) as *mut u32;
const SYS_TIMER_COMPARE_1: *mut u32 = (SYS_TIMER_BASE + 0x10) as *mut u32;

pub struct SystemTimer;

impl SystemTimer {
    pub fn init() {
        register_irq(SystemTimer::handle_irq);

        unsafe {
            let value = ptr::read_volatile(SYS_TIMER_COUNT_LOW as *mut u32) + 200000;
            ptr::write_volatile(SYS_TIMER_COMPARE_1 as *mut u32, value);
        }
    }

    pub fn reset() {
        unsafe {
            core::ptr::write_volatile(SYS_TIMER_CONTROL as *mut u32, 1 << 1);
            let value = core::ptr::read_volatile(SYS_TIMER_COUNT_LOW as *mut u32) + 200000;
            core::ptr::write_volatile(SYS_TIMER_COMPARE_1 as *mut u32, value);
        }
    }

    fn handle_irq() {
        SystemTimer::reset();
    }
}

