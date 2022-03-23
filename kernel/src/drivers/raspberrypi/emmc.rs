
use core::ptr;

use crate::printkln;
use crate::types::BlockDriver;
use crate::errors::KernelError;

//use super::gpio;


const MMC_RESP_COMMAND_COMPLETE: u32     = 1 << 31;


#[derive(Debug)]
enum Command {
    GoIdle,             // CMD0
    SendCID,            // CMD2
    SendRelAddr,        // CMD3
    CardSelect,         // CMD7
    SendIfCond,         // CMD8
    StopTransmission,   // CMD12
    ReadSingle,         // CMD17
    SendOpCond,         // ACMD41
    AppCommand,         // CMD55
}

pub struct EmmcDevice;

impl EmmcDevice {
    pub fn init() -> Result<(), KernelError> {
        EmmcHost::init()?;

        EmmcHost::send_command(Command::GoIdle, 0)?;

        EmmcHost::send_command(Command::SendIfCond, 0x000001AA)?;
        loop {
            EmmcHost::send_command(Command::AppCommand, 0)?;
            let response = EmmcHost::send_command(Command::SendOpCond, 0x51ff8000)?;
            if response & MMC_RESP_COMMAND_COMPLETE != 0 {
                break;
            }
        }

        EmmcHost::send_command(Command::SendCID, 0)?;
        let card = EmmcHost::send_command(Command::SendRelAddr, 0)?;
        EmmcHost::send_command(Command::CardSelect, card)?;

        Ok(())
    }

    pub fn read_data(buffer: &mut [u8], offset: usize) -> Result<(), KernelError> {
        let blocksize = 256;
        let mut i = 0;
        let mut len = buffer.len();

        while len > 0 {
            EmmcDevice::read_segment(offset + i, &mut buffer[i..(i + blocksize)])?;
            len -= blocksize;
            i += blocksize;
        }

        Ok(())
    }

    fn read_segment(offset: usize, buffer: &mut [u8]) -> Result<(), KernelError> {
        EmmcHost::setup_data_transfer(1, buffer.len())?;
        EmmcHost::send_command(Command::ReadSingle, offset as u32)?;

        EmmcHost::read_data(buffer)?;

        EmmcHost::send_command(Command::StopTransmission, 0)?;

        Ok(())
    }
}

impl BlockDriver for EmmcDevice {
    fn init(&self) -> Result<(), KernelError> {
        EmmcDevice::init()
    }

    fn read(&self, buffer: &mut [u8], offset: usize) -> Result<(), KernelError> {
        EmmcDevice::read_data(buffer, offset)
    }

    fn write(&self, _buffer: &[u8], _offset: usize) -> Result<(), KernelError> {
        Err(KernelError::PermissionNotAllowed)
    }
}



const EMMC1_BASE_ADDR: u64 = 0x3F30_0000;

//const EMMC1_ARG2: *mut u32              = (EMMC1_BASE_ADDR + 0x00) as *mut u32;
const EMMC1_BLOCK_COUNT_SIZE: *mut u32  = (EMMC1_BASE_ADDR + 0x04) as *mut u32;
const EMMC1_ARG1: *mut u32              = (EMMC1_BASE_ADDR + 0x08) as *mut u32;
const EMMC1_COMMAND: *mut u32           = (EMMC1_BASE_ADDR + 0x0C) as *mut u32;
const EMMC1_RESPONSE0: *mut u32         = (EMMC1_BASE_ADDR + 0x10) as *mut u32;
const EMMC1_RESPONSE1: *mut u32         = (EMMC1_BASE_ADDR + 0x14) as *mut u32;
const EMMC1_RESPONSE2: *mut u32         = (EMMC1_BASE_ADDR + 0x18) as *mut u32;
const EMMC1_RESPONSE3: *mut u32         = (EMMC1_BASE_ADDR + 0x1C) as *mut u32;
const EMMC1_DATA: *mut u32              = (EMMC1_BASE_ADDR + 0x20) as *mut u32;
const EMMC1_STATUS: *mut u32            = (EMMC1_BASE_ADDR + 0x24) as *mut u32;
const EMMC1_HOST_CONTROL0: *mut u32     = (EMMC1_BASE_ADDR + 0x28) as *mut u32;
const EMMC1_HOST_CONTROL1: *mut u32     = (EMMC1_BASE_ADDR + 0x2C) as *mut u32;
const EMMC1_INTERRUPT_FLAGS: *mut u32   = (EMMC1_BASE_ADDR + 0x30) as *mut u32;
const EMMC1_INTERRUPT_MASK: *mut u32    = (EMMC1_BASE_ADDR + 0x34) as *mut u32;
const EMMC1_INTERRUPT_ENABLE: *mut u32  = (EMMC1_BASE_ADDR + 0x38) as *mut u32;
//const EMMC1_HOST_CONTROL2: *mut u32     = (EMMC1_BASE_ADDR + 0x3C) as *mut u32;

const EMMC1_HC1_CLOCK_STABLE: u32       = 1 << 1;
const EMMC1_HC1_RESET_HOST: u32         = 1 << 24;

const EMMC1_STA_COMMAND_INHIBIT: u32    = 1 << 0;
const EMMC1_STA_DATA_INHIBIT: u32       = 1 << 1;

const EMMC1_INT_COMMAND_DONE: u32       = 1 << 0;
const EMMC1_INT_DATA_DONE: u32          = 1 << 1;
const EMMC1_INT_READ_READY: u32         = 1 << 5;
const EMMC1_INT_ANY_ERROR: u32          = 0x17F << 16;


pub struct EmmcHost;

impl EmmcHost {
    fn init() -> Result<(), KernelError> {
        //gpio::enable_emmc1();

        unsafe {
            // Reset all host circuitry
            ptr::write_volatile(EMMC1_HOST_CONTROL0, 0);
            ptr::write_volatile(EMMC1_HOST_CONTROL1, EMMC1_HC1_RESET_HOST);

            // Wait for reset to clear
            while (ptr::read_volatile(EMMC1_HOST_CONTROL1) & EMMC1_HC1_RESET_HOST) != 0 { }

            // Configure the clock
            ptr::write_volatile(EMMC1_HOST_CONTROL1, 0x000E_6805);
            wait_until_set(EMMC1_HOST_CONTROL1, EMMC1_HC1_CLOCK_STABLE).unwrap();

            ptr::write_volatile(EMMC1_INTERRUPT_ENABLE, 0xffff_ffff);
            ptr::write_volatile(EMMC1_INTERRUPT_MASK, 0xffff_ffff);
        }

        Ok(())
    }

    fn send_command(cmd: Command, arg1: u32) -> Result<u32, KernelError> {
        unsafe {
            wait_until_clear(EMMC1_STATUS, EMMC1_STA_COMMAND_INHIBIT)?;

            printkln!("mmc: sending command {:?} {:x}", cmd, arg1);
            ptr::write_volatile(EMMC1_ARG1, arg1);
            ptr::write_volatile(EMMC1_COMMAND, command_code(cmd));

            // TODO this causes it to hang, but I'm not sure why.  It was done by the bare metal raspi C project
            //ptr::write_volatile(EMMC1_INTERRUPT_FLAGS, ptr::read_volatile(EMMC1_INTERRUPT_FLAGS));

            wait_until_set(EMMC1_INTERRUPT_FLAGS, EMMC1_INT_COMMAND_DONE | EMMC1_INT_ANY_ERROR)?;

            let flags = ptr::read_volatile(EMMC1_INTERRUPT_FLAGS);
            if flags & EMMC1_INT_ANY_ERROR != 0 {
                printkln!("mmc: error occurred: {:x}", flags);
                // TODO this is temporary until the error issue is solved
                Ok(0)
            } else {
                let r0 = ptr::read_volatile(EMMC1_RESPONSE0);
                let r1 = ptr::read_volatile(EMMC1_RESPONSE1);
                let r2 = ptr::read_volatile(EMMC1_RESPONSE2);
                let r3 = ptr::read_volatile(EMMC1_RESPONSE3);
                printkln!("mmc: received response {:x} {:x} {:x} {:x}", r0, r1, r2, r3);
                Ok(r0)
            }
        }
    }

    fn setup_data_transfer(numblocks: usize, blocksize: usize) -> Result<(), KernelError> {
        wait_until_clear(EMMC1_STATUS, EMMC1_STA_DATA_INHIBIT)?;

        unsafe {
            ptr::write_volatile(EMMC1_BLOCK_COUNT_SIZE, ((numblocks << 16) | blocksize) as u32);
        }
        Ok(())
    }

    fn read_data(data: &mut [u8]) -> Result<(), KernelError> {
        wait_until_set(EMMC1_INTERRUPT_FLAGS, EMMC1_INT_READ_READY)?;

        for i in (0..data.len()).step_by(4) {
            let value = unsafe { ptr::read_volatile(EMMC1_DATA) };

            let bytes = if data.len() - i < 4 { data.len() - i } else { 4 };
            for j in 0..bytes {
                data[i + j] = (value >> (j * 8)) as u8;
            }
        }

        Ok(())
    }
}

const fn command_code(cmd: Command) -> u32 {
    match cmd {
        Command::GoIdle             => 0x00000000,
        Command::SendIfCond         => 0x08020000,
        Command::StopTransmission   => 0x0C030000,
        Command::SendOpCond         => 0x29020000,
        Command::ReadSingle         => 0x11220010,
        Command::AppCommand         => 0x37000000,
        Command::SendCID            => 0x02010000,
        Command::SendRelAddr        => 0x03020000,
        Command::CardSelect         => 0x07030000,
    }
}

fn wait_until_set(reg: *const u32, mask: u32) -> Result<(), KernelError> {
    for _ in 0..1000 {
        unsafe {
            if (ptr::read_volatile(reg) & mask) != 0 {
                return Ok(());
            }
        }
    }
    Err(KernelError::DeviceTimeout)
}

fn wait_until_clear(reg: *const u32, mask: u32) -> Result<(), KernelError> {
    for _ in 0..1000 {
        unsafe {
            if (ptr::read_volatile(reg) & mask) == 0 {
                return Ok(());
            }
        }
    }
    Err(KernelError::DeviceTimeout)
}

