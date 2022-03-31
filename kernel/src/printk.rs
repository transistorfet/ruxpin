
use core::fmt;
use core::fmt::Write;

use crate::sync::Spinlock;
use crate::types::CharDriver;


static mut CONSOLE_DEVICE: Option<&Spinlock<dyn CharDriver>> = None;

pub fn set_console_device(dev: &'static Spinlock<dyn CharDriver>) {
    unsafe {
        CONSOLE_DEVICE = Some(dev);
    }
}

pub fn printk_args(args: fmt::Arguments) {
    unsafe {
        CONSOLE_DEVICE.as_mut().unwrap().lock().write_fmt(args).unwrap()
    }
    //crate::config::console::ConsoleDevice::new().write_fmt(args).unwrap()
}

#[macro_export]
macro_rules! printk {
    ($($args:tt)*) => ({
        $crate::printk::printk_args(format_args!($($args)*));
    })
}

#[macro_export]
macro_rules! printkln {
    ($($args:tt)*) => ({
        $crate::printk::printk_args(format_args!($($args)*));
        $crate::printk::printk_args(format_args!("\n"));
    })
}

impl fmt::Write for dyn CharDriver {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s.as_bytes()).unwrap();
        Ok(())
    }
}

pub unsafe fn printk_dump(mut ptr: *const u8, mut size: usize) {
    while size > 0 {
        printk!("{:#010x}: ", ptr as u64);
        for i in 0..16 {
            printk!("{:02x} ", *ptr.offset(i));
            size -= 1;
            if size == 0 {
                break;
            }
        }
        ptr = ptr.offset(16);
        printkln!("");
    }
}

