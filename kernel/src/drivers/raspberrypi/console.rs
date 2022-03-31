
use core::fmt;
use core::ptr;

use crate::arch::sync::{Spinlock, SpinlockGuard};
use crate::errors::KernelError;
use crate::types::{FileFlags, CharDriver};


const PL011_BASE: u64 = 0x3F20_1000;

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


static DEFAULT_CONSOLE: Spinlock<ConsoleDevice> = Spinlock::new(ConsoleDevice { opens: 0 });

pub struct ConsoleDevice {
    opens: u32,
}

impl CharDriver for ConsoleDevice {
    fn init(&mut self) -> Result<(), KernelError> {
        self.setup_basic_io();
        Ok(())
    }

    fn open(&mut self, _mode: FileFlags) -> Result<(), KernelError> {
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


impl ConsoleDevice {
    pub fn setup_basic_io(&self) {
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

    pub fn write(&self, s: &str) {
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

/*
impl fmt::Write for ConsoleDevice {
    fn write_str(&mut self, s: &str) -> fmt::Result { 
        for ch in s.chars() {
            self.put_char(ch as u8);
        }

        //self.flush();
        Ok(())
    }
}

pub fn get_console<'a>() -> SpinlockGuard<'a, impl fmt::Write> {
    DEFAULT_CONSOLE.lock()
}
*/

pub fn get_console_device<'a>() -> SpinlockGuard<'a, impl CharDriver> {
    DEFAULT_CONSOLE.lock()
}

pub fn get_console_device_spinlock() -> &'static Spinlock<dyn CharDriver> {
    &DEFAULT_CONSOLE
}

