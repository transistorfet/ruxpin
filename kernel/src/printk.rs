
use core::fmt;
use core::fmt::Write;


#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum LogLevel {
    Panic,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
    Trace,
}

static mut CONSOLE_LOG_LEVEL: LogLevel = LogLevel::Info;
static mut CONSOLE_DEVICE: ConsoleDevice = ConsoleDevice(null_printk);

fn null_printk(_: &str) {
    // Do Nothing
}

pub fn set_console_device(func: fn(&str)) {
    unsafe {
        CONSOLE_DEVICE = ConsoleDevice(func);
    }
}

#[inline(always)]
pub fn printk_args(loglevel: LogLevel, args: fmt::Arguments) {
    unsafe {
        if loglevel <= CONSOLE_LOG_LEVEL {
            CONSOLE_DEVICE.write_fmt(args).unwrap();
        }
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
    ($loglevel:expr, $($args:tt)*) => ({
        $crate::printk::printk_args($loglevel, format_args!($($args)*));
    })
}

#[macro_export]
macro_rules! printkln {
    ($loglevel:expr, $($args:tt)*) => ({
        $crate::printk::printk_args($loglevel, format_args!($($args)*));
        $crate::printk::printk_args($loglevel, format_args!("\n"));
    })
}

#[macro_export]
macro_rules! error {
    ($($args:tt)*) => ({
        use $crate::printk::LogLevel;
        $crate::printkln!(LogLevel::Error, $($args)*);
    })
}

#[macro_export]
macro_rules! warning {
    ($($args:tt)*) => ({
        use $crate::printk::LogLevel;
        $crate::printkln!(LogLevel::Warning, $($args)*);
    })
}

#[macro_export]
macro_rules! notice {
    ($($args:tt)*) => ({
        use $crate::printk::LogLevel;
        $crate::printkln!(LogLevel::Notice, $($args)*);
    })
}

#[macro_export]
macro_rules! info {
    ($($args:tt)*) => ({
        use $crate::printk::LogLevel;
        $crate::printkln!(LogLevel::Info, $($args)*);
    })
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => ({
        use $crate::printk::LogLevel;
        $crate::printkln!(LogLevel::Debug, $($args)*);
    })
}

#[macro_export]
macro_rules! trace {
    ($($args:tt)*) => ({
        use $crate::printk::LogLevel;
        $crate::printkln!(LogLevel::Trace, $($args)*);
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
        printk!(LogLevel::Info, "{:#010x}: ", addr);
        let ptr = addr as *const u8;
        for i in 0..16 {
            printk!(LogLevel::Info, "{:02x} ", *ptr.add(i));
            size -= 1;
            if size == 0 {
                break;
            }
        }
        addr += 16;
        printkln!(LogLevel::Info, "");
    }
}

