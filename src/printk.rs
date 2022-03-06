
#[macro_export]
macro_rules! printk {
    ($($args:tt)*) => ($crate::arch::console::get_console().write_fmt(format_args!($($args)*)).unwrap())
}

