
use core::fmt;

use crate::mm::__KERNEL_VIRTUAL_BASE_ADDR;

static SERIAL_OUT: u64 = __KERNEL_VIRTUAL_BASE_ADDR + 0x3F20_1000;

pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result { 
        for ch in s.chars() {
            unsafe {
                core::ptr::write_volatile(SERIAL_OUT as *mut u8, ch as u8);
            }
        }
        Ok(())
    }
}

pub fn get_console() -> impl fmt::Write {
    Console {}
}

