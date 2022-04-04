
use core::ptr;

use alloc::boxed::Box;

use ruxpin_api::types::{OpenFlags, DeviceID};

use crate::errors::KernelError;
use crate::tty::{self, CharOperations};
use crate::printk::set_console_device;
use crate::arch::types::KernelVirtualAddress;
use crate::misc::deviceio::DeviceRegisters;


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


const PL011: DeviceRegisters<u32> = DeviceRegisters::new(KernelVirtualAddress::new(0x3F20_1000));

mod registers {
    pub const DATA: usize               = 0x00;
    pub const FLAGS: usize              = 0x18;
    pub const BAUD_INTEGER: usize       = 0x24;
    pub const BAUD_FRACTIONAL: usize    = 0x28;
    pub const LINE_CONTROL: usize       = 0x2C;
    pub const CONTROL: usize            = 0x30;
    pub const INTERRUPT_MASK: usize     = 0x38;
    pub const INTERRUPT_CLEAR: usize    = 0x44;
}

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
            PL011.set(registers::CONTROL, 0);

            // Clear all interrupt flags
            PL011.set(registers::INTERRUPT_CLEAR, 0x3FF);
            // Mask all the interrupts
            PL011.set(registers::INTERRUPT_MASK, 0x3FF);

            // Disable the FIFO before changing the baud rate
            let lcr = PL011.get(registers::LINE_CONTROL);
            PL011.set(registers::LINE_CONTROL, lcr & !PL011_LC_FIFO_ENABLE);

            // Set the speed to 921_600 baud (to match MiniLoad from https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials)
            PL011.set(registers::BAUD_INTEGER, 3);
            PL011.set(registers::BAUD_FRACTIONAL, 16);

            // Enable FIFO and configure for 8 bits, 1 stop bit, no parity
            PL011.set(registers::LINE_CONTROL, (0b11 << 5) | PL011_LC_FIFO_ENABLE);

            // Enable UART (TX only)
            PL011.set(registers::CONTROL, PL011_CTL_UART_ENABLE | PL011_CTL_TX_ENABLE | PL011_CTL_RX_ENABLE);
        }
    }

    pub fn put_char(&self, byte: u8) {
        unsafe {
            while (PL011.get(registers::FLAGS) & PL011_FLAGS_TX_FIFO_FULL) != 0 { }
            PL011.set(registers::DATA, byte as u32);
        }
    }

    pub fn get_char(&self) -> Option<u8> {
        unsafe {
            if PL011.get(registers::FLAGS) & PL011_FLAGS_RX_FIFO_EMPTY == 0 {
                Some(PL011.get(registers::DATA) as u8)
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
            while (PL011.get(registers::FLAGS) & PL011_FLAGS_TX_FIFO_EMPTY) == 0 { }
        }
    }
}

