
use core::ptr;

use alloc::boxed::Box;

use ruxpin_api::types::{OpenFlags, DeviceID};

use crate::errors::KernelError;
use crate::tty::{self, CharOperations};
use crate::printk::set_console_device;

static mut SAFE_CONSOLE: PL011Device = PL011Device { opens: 0 };
static mut NORMAL_CONSOLE: DeviceID = DeviceID(0, 0);


pub fn init() -> Result<(), KernelError> {
    let driver_id = tty::register_tty_driver("console")?;
    let console = PL011Device { opens: 0 };
    console.init();
    let subdevice_id = tty::register_tty_device(driver_id, Box::new(console))?;
    set_normal_console(DeviceID(driver_id, subdevice_id));

    Ok(())
}

pub fn set_safe_console() {
    set_console_device(safe_console_print);
}

fn safe_console_print(s: &str) {
    unsafe {
        SAFE_CONSOLE.print(s);
    }
}

pub fn set_normal_console(device: DeviceID) {
    unsafe {
        NORMAL_CONSOLE = device;
    }
    set_console_device(normal_console_print);
}

fn normal_console_print(s: &str) {
    unsafe {
        tty::write(NORMAL_CONSOLE, s.as_bytes()).unwrap();
    }
}


pub struct PL011Device {
    opens: u32,
}

impl CharOperations for PL011Device {
    fn open(&mut self, _mode: OpenFlags) -> Result<(), KernelError> {
        self.opens += 1;
        Ok(())
    }

    fn close(&mut self) -> Result<(), KernelError> {
        if self.opens == 0 {
            return Err(KernelError::FileNotOpen);
        }

        self.opens -= 1;
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, KernelError> {
        let mut i = 0;
        while i < buffer.len() {
            if let Some(byte) = self.get_char() {
                buffer[i] = byte;
            } else {
                break;
            }
            i += 1;
        }
        Ok(i)
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, KernelError> {
        for byte in buffer {
            self.put_char(*byte);
        }
        Ok(buffer.len())
    }
}


const PL011_BASE: u64 = 0xFFFF_0000_3F20_1000;

const PL011_DATA: *mut u32              = (PL011_BASE + 0x00) as *mut u32;
const PL011_FLAGS: *mut u32             = (PL011_BASE + 0x18) as *mut u32;
const PL011_BAUD_INTEGER: *mut u32      = (PL011_BASE + 0x24) as *mut u32;
const PL011_BAUD_FRACTIONAL: *mut u32   = (PL011_BASE + 0x28) as *mut u32;
const PL011_LINE_CONTROL: *mut u32      = (PL011_BASE + 0x2C) as *mut u32;
const PL011_CONTROL: *mut u32           = (PL011_BASE + 0x30) as *mut u32;
const PL011_INTERRUPT_MASK: *mut u32    = (PL011_BASE + 0x38) as *mut u32;
const PL011_INTERRUPT_CLEAR: *mut u32   = (PL011_BASE + 0x44) as *mut u32;

const PL011_FLAGS_RX_FIFO_EMPTY: u32    = 1 << 4;
const PL011_FLAGS_TX_FIFO_FULL: u32     = 1 << 5;
const PL011_FLAGS_TX_FIFO_EMPTY: u32    = 1 << 7;

const PL011_CTL_UART_ENABLE: u32        = 1 << 0;
const PL011_CTL_TX_ENABLE: u32          = 1 << 8;
const PL011_CTL_RX_ENABLE: u32          = 1 << 9;

const PL011_LC_FIFO_ENABLE: u32         = 1 << 4;


impl PL011Device {
    pub fn init(&self) {
        unsafe {
            // Disable UART
            ptr::write_volatile(PL011_CONTROL, 0);

            // Clear all interrupt flags
            ptr::write_volatile(PL011_INTERRUPT_CLEAR, 0x3FF);
            // Mask all the interrupts
            ptr::write_volatile(PL011_INTERRUPT_MASK, 0x3FF);

            // Disable the FIFO before changing the baud rate
            let lcr = ptr::read_volatile(PL011_LINE_CONTROL);
            ptr::write_volatile(PL011_LINE_CONTROL, lcr & !PL011_LC_FIFO_ENABLE);

            // Set the speed to 921_600 baud (to match MiniLoad from https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials)
            ptr::write_volatile(PL011_BAUD_INTEGER, 3);
            ptr::write_volatile(PL011_BAUD_FRACTIONAL, 16);

            // Enable FIFO and configure for 8 bits, 1 stop bit, no parity
            ptr::write_volatile(PL011_LINE_CONTROL, (0b11 << 5) | PL011_LC_FIFO_ENABLE);

            // Enable UART (TX only)
            ptr::write_volatile(PL011_CONTROL, PL011_CTL_UART_ENABLE | PL011_CTL_TX_ENABLE | PL011_CTL_RX_ENABLE);
        }
    }

    pub fn put_char(&self, byte: u8) {
        unsafe {
            while (ptr::read_volatile(PL011_FLAGS) & PL011_FLAGS_TX_FIFO_FULL) != 0 { }
            ptr::write_volatile(PL011_DATA, byte as u32);
        }
    }

    pub fn get_char(&self) -> Option<u8> {
        unsafe {
            if ptr::read_volatile(PL011_FLAGS) & PL011_FLAGS_RX_FIFO_EMPTY == 0 {
                Some(ptr::read_volatile(PL011_DATA) as u8)
            } else {
                None
            }
        }
    }

    pub fn print(&self, s: &str) {
        for ch in s.chars() {
            self.put_char(ch as u8);
        }
    }

    #[allow(dead_code)]
    pub fn flush(&self) {
        unsafe {
            while (ptr::read_volatile(PL011_FLAGS) & PL011_FLAGS_TX_FIFO_EMPTY) == 0 { }
        }
    }
}

