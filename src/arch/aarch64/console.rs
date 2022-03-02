
use core::fmt;

const SERIAL_OUT: *mut u8 = 0x3F20_1000 as *mut u8;

pub struct SimpleConsole;

impl fmt::Write for SimpleConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result { 
        for ch in s.chars() {
            unsafe {
                core::ptr::write_volatile(SERIAL_OUT, ch as u8);
            }
        }
        Ok(())
    }
}
