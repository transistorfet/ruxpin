
use core::fmt;
use core::fmt::Write;

pub fn printk_args(args: fmt::Arguments) {
    crate::arch::console::get_console().write_fmt(args).unwrap()
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
        $crate::printk::printk_args(format_args!($($args)*));
        $crate::arch::console::get_console().write_str("\n").unwrap();
    })
}

pub unsafe fn printk_dump(mut ptr: *mut i8, mut size: usize) {
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

