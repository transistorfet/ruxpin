
use core::fmt;
use core::ptr;
use core::arch::asm;


const PL011_BASE: u64 = 0x3F20_1000;

const PL011_DATA: *mut u32              = (PL011_BASE + 0x00) as *mut u32;
const PL011_FLAGS: *mut u32             = (PL011_BASE + 0x18) as *mut u32;
const PL011_BAUD_INTEGER: *mut u32      = (PL011_BASE + 0x24) as *mut u32;
const PL011_BAUD_FRACTIONAL: *mut u32   = (PL011_BASE + 0x28) as *mut u32;
const PL011_LINE_CONTROL: *mut u32      = (PL011_BASE + 0x2C) as *mut u32;
const PL011_CONTROL: *mut u32           = (PL011_BASE + 0x30) as *mut u32;
const PL011_INTERRUPT_CLEAR: *mut u32   = (PL011_BASE + 0x44) as *mut u32;

const PL011_FLAGS_TX_FIFO_FULL: u32     = 1 << 5;
const PL011_FLAGS_TX_FIFO_EMPTY: u32    = 1 << 7;

const PL011_CTL_UART_ENABLE: u32        = 1 << 0;
const PL011_CTL_TX_ENABLE: u32          = 1 << 8;
const PL011_CTL_RX_ENABLE: u32          = 1 << 9;

const PL011_LC_FIFO_ENABLE: u32         = 1 << 4;


pub struct Console;

impl Console {
    pub fn init() {
        unsafe {
            // Disable UART
            ptr::write_volatile(PL011_CONTROL, 0);

            // Clear all interrupt flags
            ptr::write_volatile(PL011_INTERRUPT_CLEAR, 0x3FF);

            // Disable the FIFO before changing the baud rate
            let lcr = ptr::read_volatile(PL011_LINE_CONTROL);
            ptr::write_volatile(PL011_LINE_CONTROL, lcr & !PL011_LC_FIFO_ENABLE);

            // Set the speed to 921_600 baud (to match MiniLoad from https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials)
            ptr::write_volatile(PL011_BAUD_INTEGER, 3);
            ptr::write_volatile(PL011_BAUD_FRACTIONAL, 16);

            // Enable FIFO and configure for 8 bits, 1 stop bit, no parity
            ptr::write_volatile(PL011_LINE_CONTROL, (0b11 << 5) | PL011_LC_FIFO_ENABLE);

            // Enable UART (TX only)
            ptr::write_volatile(PL011_CONTROL, PL011_CTL_UART_ENABLE | PL011_CTL_TX_ENABLE);
        }
    }

    pub fn flush(&self) {
        unsafe {
            while (ptr::read_volatile(PL011_FLAGS) & PL011_FLAGS_TX_FIFO_EMPTY) == 0 { }
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result { 
        for ch in s.chars() {
            unsafe {
                while (ptr::read_volatile(PL011_FLAGS) & PL011_FLAGS_TX_FIFO_FULL) != 0 { }

                ptr::write_volatile(PL011_DATA, ch as u32);
            }
        }

        //self.flush();
        Ok(())
    }
}

pub fn get_console() -> impl fmt::Write {
    Console {}
}

