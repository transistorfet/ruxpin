
use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;

use ruxpin_types::{OpenFlags, DeviceID};

use ruxpin_kernel::irqs;
use ruxpin_kernel::notice;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::tty::{self, CharOperations};
use ruxpin_kernel::printk::set_console_device;
use ruxpin_kernel::arch::KernelVirtualAddress;
use ruxpin_kernel::misc::deviceio::DeviceRegisters;


static PL011_DRIVER_NAME: &'static str = "console";

static mut PL011_RX: Option<PL011Rx> = None;
static mut TTY_CONSOLE: Option<DeviceID> = None;


pub fn register() -> Result<(), KernelError> {
    notice!("{}: initializing", PL011_DRIVER_NAME);
    let driver_id = tty::register_tty_driver(PL011_DRIVER_NAME)?;
    let console = PL011Device::new();
    init();
    let subdevice_id = tty::register_tty_device(driver_id, Box::new(console))?;
    set_normal_console(DeviceID(driver_id, subdevice_id));

    Ok(())
}

pub fn set_safe_console() {
    set_console_device(safe_console_print);
}

fn safe_console_print(s: &str) {
    print(s);
}

pub fn set_normal_console(device: DeviceID) {
    unsafe {
        PL011_RX = Some(PL011Rx::new());
        TTY_CONSOLE = Some(device);
    }
    set_console_device(normal_console_print);
}

fn normal_console_print(s: &str) {
    unsafe {
        tty::write(TTY_CONSOLE.unwrap(), s.as_bytes()).unwrap();
    }
}


pub struct PL011Device {
    opens: u32,
}

impl PL011Device {
    pub const fn new() -> Self {
        Self {
            opens: 0,
        }
    }
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
            if let Some(byte) = unsafe { PL011_RX.as_mut().unwrap().get_char_buffered() } {
                buffer[i] = byte;
            } else {
                if i == 0 {
                    //use crate::tasklets::schedule_tasklet;
                    //schedule_tasklet(Box::new(|| { process::suspend_current_process(); Ok(()) }));
                    //process::suspend_current_process();
                    //return Err(KernelError::SuspendProcess);
                }
                break;
            }
            i += 1;
        }
        Ok(i)
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, KernelError> {
        for byte in buffer {
            put_char(*byte);
        }
        Ok(buffer.len())
    }
}

const PL011_IRQ: usize = 57;
const PL011: DeviceRegisters<u32> = DeviceRegisters::new(KernelVirtualAddress::new(0x3F20_1000));

mod registers {
    pub const DATA: usize               = 0x00;
    pub const FLAGS: usize              = 0x18;
    pub const BAUD_INTEGER: usize       = 0x24;
    pub const BAUD_FRACTIONAL: usize    = 0x28;
    pub const LINE_CONTROL: usize       = 0x2C;
    pub const CONTROL: usize            = 0x30;
    pub const INTERRUPT_MASK: usize     = 0x38;
    pub const INTERRUPT_STATUS: usize   = 0x3C;
    pub const INTERRUPT_CLEAR: usize    = 0x44;
}

const PL011_FLAGS_RX_FIFO_EMPTY: u32    = 1 << 4;
const PL011_FLAGS_TX_FIFO_FULL: u32     = 1 << 5;
const PL011_FLAGS_TX_FIFO_EMPTY: u32    = 1 << 7;

const PL011_CTL_UART_ENABLE: u32        = 1 << 0;
const PL011_CTL_TX_ENABLE: u32          = 1 << 8;
const PL011_CTL_RX_ENABLE: u32          = 1 << 9;

const PL011_LC_FIFO_ENABLE: u32         = 1 << 4;

//const PL011_INT_RX_TIMEOUT: u32         = 1 << 6;
//const PL011_INT_TX_READY: u32           = 1 << 5;
const PL011_INT_RX_READY: u32           = 1 << 4;
const PL011_INT_ALL: u32                = 0x3FF;

const PL011_RX_QUEUE_SIZE: usize        = 32;


fn init() {
    unsafe {
        irqs::disable_irq(PL011_IRQ);

        // Disable UART
        PL011.set(registers::CONTROL, 0);

        // Clear all interrupt flags
        PL011.set(registers::INTERRUPT_CLEAR, PL011_INT_ALL);
        // Mask all the interrupts
        PL011.set(registers::INTERRUPT_MASK, 0x0);

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

        // Enable the RX interrupt
        PL011.set(registers::INTERRUPT_MASK, PL011_INT_RX_READY);
        irqs::register_irq(PL011_IRQ, handle_irq_pl011).unwrap();
        irqs::enable_irq(PL011_IRQ);
    }
}

fn put_char(byte: u8) {
    unsafe {
        while (PL011.get(registers::FLAGS) & PL011_FLAGS_TX_FIFO_FULL) != 0 { }
        PL011.set(registers::DATA, byte as u32);
    }
}

fn get_char() -> Option<u8> {
    unsafe {
        if PL011.get(registers::FLAGS) & PL011_FLAGS_RX_FIFO_EMPTY == 0 {
            Some(PL011.get(registers::DATA) as u8)
        } else {
            None
        }
    }
}

fn print(s: &str) {
    for ch in s.chars() {
        put_char(ch as u8);
    }
}

#[allow(dead_code)]
fn flush() {
    unsafe {
        while (PL011.get(registers::FLAGS) & PL011_FLAGS_TX_FIFO_EMPTY) == 0 { }
    }
}


struct PL011Rx {
    buffer: VecDeque<u8>,
}

impl PL011Rx {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(PL011_RX_QUEUE_SIZE),
        }
    }

    fn get_char_buffered(&mut self) -> Option<u8> {
        self.buffer.pop_front()
    }
}

pub fn handle_irq_pl011() {
    unsafe {
        let status = PL011.get(registers::INTERRUPT_STATUS);
        PL011.set(registers::INTERRUPT_CLEAR, PL011_INT_ALL);

        if status & PL011_INT_RX_READY != 0 {
            while let Some(ch) = get_char() {
                //crate::debug!(">>> {}", ch);

                PL011_RX.as_mut().unwrap().buffer.push_back(ch);

                if let Some(device_id) = TTY_CONSOLE {
                    tty::schedule_update(device_id);
                }
            }
        }
    }
}

