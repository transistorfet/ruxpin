
use core::fmt;
use core::fmt::Write;


static mut CONSOLE_DEVICE: ConsoleDevice = ConsoleDevice(null_printk);

fn null_printk(_: &str) {
    // Do Nothing
}

pub fn set_console_device(func: fn(&str)) {
    unsafe {
        CONSOLE_DEVICE = ConsoleDevice(func);
    }
}

pub fn printk_args(args: fmt::Arguments) {
    unsafe {
        CONSOLE_DEVICE.write_fmt(args).unwrap();
    }
}

struct ConsoleDevice(fn(&str));

impl Write for ConsoleDevice {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        //tty::write(self.0, s.as_bytes()).unwrap();
        self.0(s);
        Ok(())
    }
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

pub fn printk_dump_slice<T>(data: &[T]) {
    let ptr = data.as_ptr() as *const u8 as u64;
    let len = data.len();
    unsafe {
        printk_dump(ptr, len);
    }
}

pub unsafe fn printk_dump(mut addr: u64, mut size: usize) {
    while size > 0 {
        printk!("{:#010x}: ", addr);
        let ptr = addr as *const u8;
        for i in 0..16 {
            printk!("{:02x} ", unsafe { *ptr.add(i) });
            size -= 1;
            if size == 0 {
                break;
            }
        }
        addr += 16;
        printkln!("");
    }
}

