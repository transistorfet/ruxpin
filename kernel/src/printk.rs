
use core::fmt;
use core::fmt::Write;

use crate::types::CharDriver;

pub fn printk_args(args: fmt::Arguments) {
    let dev: &mut dyn CharDriver = &mut *crate::config::console::get_console_device();
    dev.write_fmt(args).unwrap()
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
        use core::fmt::Write;
        use crate::types::CharDriver;
        $crate::printk::printk_args(format_args!($($args)*));
        let dev: &mut dyn CharDriver = &mut *$crate::config::console::get_console_device();
        dev.write_str("\n").unwrap();
    })
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

