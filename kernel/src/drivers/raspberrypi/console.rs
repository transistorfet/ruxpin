
use core::fmt;
use core::ptr;

const SERIAL_OUT: *mut u8 = 0x3F20_1000 as *mut u8;

pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result { 
        for ch in s.chars() {
            unsafe {
                ptr::write_volatile(SERIAL_OUT, ch as u8);
            }
        }
        Ok(())
    }
}

pub fn get_console() -> impl fmt::Write {
    Console {}
}

