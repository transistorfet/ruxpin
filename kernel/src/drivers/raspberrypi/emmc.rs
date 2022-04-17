
use alloc::boxed::Box;

use crate::block;
use crate::printkln;
use crate::errors::KernelError;
use crate::block::BlockOperations;
use crate::arch::types::KernelVirtualAddress;
use crate::misc::deviceio::DeviceRegisters;
use crate::block::partition::Partition;

use ruxpin_api::types::{OpenFlags, DeviceID};

//use super::gpio;


static EMMC_DRIVER_NAME: &'static str = "sd";

pub struct EmmcDevice {
    base: u64,
    size: u64,
}

#[derive(Copy, Clone, PartialEq)]
enum ReadWrite {
    Read,
    Write,
}

impl EmmcDevice {
    pub fn register() -> Result<(), KernelError> {
        printkln!("{}: initializing", EMMC_DRIVER_NAME);
        let driver_id = block::register_block_driver(EMMC_DRIVER_NAME)?;
        let device_id = block::register_block_device(driver_id, Box::new(EmmcDevice::new(0, 0)))?;
        let raw_device = DeviceID(driver_id, device_id);

        let mut buffer = [0; 512];
        block::open(raw_device, OpenFlags::ReadOnly)?;
        block::read(raw_device, &mut buffer, 0)?;
        block::close(raw_device)?;

        if let Some(iter) = Partition::read_mbr_partition_table_iter(&buffer) {
            for (i, partition) in iter.enumerate() {
                printkln!("{}: found partition {} at {:x}, {} MiB", EMMC_DRIVER_NAME, i, partition.base, partition.size / 2048);
                block::register_block_device(driver_id, Box::new(EmmcDevice::new(partition.base as u64 * 512, partition.size as u64 * 512)))?;
            }
        } else {
            printkln!("{}: no partition table found", EMMC_DRIVER_NAME);
        }

        Ok(())
    }

    fn new(base: u64, size: u64) -> Self {
        Self {
            base,
            size,
        }
    }
}


impl BlockOperations for EmmcDevice {
    fn open(&mut self, _mode: OpenFlags) -> Result<(), KernelError> {
        EmmcDevice::init()
    }

    fn close(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn read(&mut self, mut buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
        if self.size != 0 && offset + buffer.len() as u64 > self.size {
            buffer = &mut buffer[..(self.size - offset as u64) as usize];
        }
        EmmcDevice::read_data(buffer, self.base + offset)
    }

    fn write(&mut self, mut buffer: &[u8], offset: u64) -> Result<usize, KernelError> {
        if self.size != 0 && offset + buffer.len() as u64 > self.size {
            buffer = &buffer[..(self.size - offset as u64) as usize];
        }
unsafe {
    crate::printk::printk_dump(buffer.as_ptr(), buffer.len());
}
        EmmcDevice::write_data(buffer, self.base + offset)
    }
}

const MMC_RESP_COMMAND_COMPLETE: u32     = 1 << 31;

#[allow(dead_code)]
#[derive(Debug)]
enum Command {
    GoIdle,             // CMD0
    SendCID,            // CMD2
    SendRelAddr,        // CMD3
    CardSelect,         // CMD7
    SendIfCond,         // CMD8
    StopTransmission,   // CMD12
    ReadSingle,         // CMD17
    ReadMultiple,       // CMD18
    WriteSingle,        // CMD24
    WriteMultiple,      // CMD25
    SendOpCond,         // ACMD41
    AppCommand,         // CMD55
}

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

        EmmcHost::finish_init()?;

        Ok(())
    }

/*
    pub fn read_data(buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
        let blocksize = 512;
        let numblocks = ceiling_div(buffer.len(), blocksize);

        printkln!("mmc: reading {} blocks of {} each at offset {:x}", numblocks, blocksize, offset);
        EmmcHost::setup_data_transfer(Command::ReadMultiple, offset / blocksize as u64, numblocks, blocksize)?;

        let mut i = 0;
        let mut len = buffer.len();
        while len > 0 {
            EmmcHost::read_data(&mut buffer[i..(i + blocksize)])?;
            len -= blocksize;
            i += blocksize;
        }

        EmmcHost::send_command(Command::StopTransmission, 0)?;

        Ok(i)
    }
*/

    pub fn read_data(buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
        let blocksize = 512;
        let mut i = 0;
        let mut len = buffer.len();

        while len > 0 {
            EmmcHost::setup_data_transfer(Command::ReadSingle, ReadWrite::Read, (offset + i as u64) / blocksize as u64, 1, blocksize)?;
            EmmcHost::read_data(&mut buffer[i..(i + blocksize)])?;
            len -= blocksize;
            i += blocksize;
        }

        Ok(i)
    }

    pub fn write_data(buffer: &[u8], offset: u64) -> Result<usize, KernelError> {
        let blocksize = 512;
        let mut i = 0;
        let mut len = buffer.len();

        while len > 0 {
            EmmcHost::setup_data_transfer(Command::WriteSingle, ReadWrite::Write, (offset + i as u64) / blocksize as u64, 1, blocksize)?;
            EmmcHost::write_data(&buffer[i..(i + blocksize)])?;
            len -= blocksize;
            i += blocksize;
        }

        Ok(i)
    }
}


const EMMC1: DeviceRegisters<u32> = DeviceRegisters::new(KernelVirtualAddress::new(0x3F30_0000));

mod registers {
    pub const ARG2: usize               = 0x00;
    pub const BLOCK_COUNT_SIZE: usize   = 0x04;
    pub const ARG1: usize               = 0x08;
    pub const COMMAND: usize            = 0x0C;
    pub const RESPONSE0: usize          = 0x10;
    pub const RESPONSE1: usize          = 0x14;
    pub const RESPONSE2: usize          = 0x18;
    pub const RESPONSE3: usize          = 0x1C;
    pub const DATA: usize               = 0x20;
    pub const STATUS: usize             = 0x24;
    pub const HOST_CONTROL0: usize      = 0x28;
    pub const HOST_CONTROL1: usize      = 0x2C;
    pub const INTERRUPT_FLAGS: usize    = 0x30;
    pub const INTERRUPT_MASK: usize     = 0x34;
    pub const INTERRUPT_ENABLE: usize   = 0x38;
}

const EMMC1_HC1_CLOCK_STABLE: u32       = 1 << 1;
const EMMC1_HC1_RESET_HOST: u32         = 1 << 24;

const EMMC1_STA_COMMAND_INHIBIT: u32    = 1 << 0;
const EMMC1_STA_DATA_INHIBIT: u32       = 1 << 1;
const EMMC1_STA_WRITE_TRANSFER: u32     = 1 << 8;
const EMMC1_STA_READ_TRANSFER: u32      = 1 << 9;

const EMMC1_INT_COMMAND_DONE: u32       = 1 << 0;
//const EMMC1_INT_DATA_DONE: u32          = 1 << 1;
const EMMC1_INT_WRITE_READY: u32        = 1 << 4;
const EMMC1_INT_READ_READY: u32         = 1 << 5;
const EMMC1_INT_ANY_ERROR: u32          = 0x17F8000;


pub struct EmmcHost;

impl EmmcHost {
    fn init() -> Result<(), KernelError> {
        //gpio::enable_emmc1();

        unsafe {
            // Reset all host circuitry
            EMMC1.set(registers::HOST_CONTROL0, 0);
            EMMC1.set(registers::HOST_CONTROL1, EMMC1_HC1_RESET_HOST);

            // Wait for reset to clear
            while (EMMC1.get(registers::HOST_CONTROL1) & EMMC1_HC1_RESET_HOST) != 0 { }

            // Configure the clock
            Self::set_clock(400_000)?;

            EMMC1.set(registers::INTERRUPT_ENABLE, 0xffff_ffff);
            EMMC1.set(registers::INTERRUPT_MASK, 0xffff_ffff);
        }

        Ok(())
    }

    fn finish_init() -> Result<(), KernelError> {
        EmmcHost::set_clock(20_000_000)?;
        Ok(())
    }

    fn set_clock(frequency: u32) -> Result<(), KernelError> {
        let divider = 41_666_666 / frequency;
        unsafe {
            // Configure the clock
            EMMC1.set(registers::HOST_CONTROL1, (0xE << 16) | ((divider & 0xFF) << 8) | ((divider & 0x300) >> 2) | 0x5);
            wait_until_set(registers::HOST_CONTROL1, EMMC1_HC1_CLOCK_STABLE).unwrap();
        }
        Ok(())
    }

    fn send_command(cmd: Command, arg1: u32) -> Result<u32, KernelError> {
        unsafe {
            wait_until_clear(registers::STATUS, EMMC1_STA_COMMAND_INHIBIT)?;

            // In order to reset the interrupt flags, they must be set to 1 (not 0), so writing it to itself will do that
            EMMC1.set(registers::INTERRUPT_FLAGS, EMMC1.get(registers::INTERRUPT_FLAGS));

            //printkln!("mmc: sending command {:?} {:x}", cmd, arg1);
            EMMC1.set(registers::ARG1, arg1);
            EMMC1.set(registers::COMMAND, command_code(cmd));

            wait_until_set(registers::INTERRUPT_FLAGS, EMMC1_INT_COMMAND_DONE | EMMC1_INT_ANY_ERROR)?;

            let flags = EMMC1.get(registers::INTERRUPT_FLAGS);
            if flags & EMMC1_INT_ANY_ERROR != 0 {
                //printkln!("mmc: error occurred: {:x}", flags);
                // TODO this is temporary until the error issue is solved
                Ok(0)
            } else {
                let r0 = EMMC1.get(registers::RESPONSE0);
                let r1 = EMMC1.get(registers::RESPONSE1);
                let r2 = EMMC1.get(registers::RESPONSE2);
                let r3 = EMMC1.get(registers::RESPONSE3);
                //printkln!("mmc: received response {:x} {:x} {:x} {:x}", r0, r1, r2, r3);
                Ok(r0)
            }
        }
    }

    fn setup_data_transfer(cmd: Command, readwrite: ReadWrite, offset: u64, numblocks: usize, blocksize: usize) -> Result<(), KernelError> {
        let rw_flag = if readwrite == ReadWrite::Read { EMMC1_STA_READ_TRANSFER } else { EMMC1_STA_WRITE_TRANSFER };
        wait_until_clear(registers::STATUS, EMMC1_STA_DATA_INHIBIT | rw_flag)?;

        unsafe {
            EMMC1.set(registers::BLOCK_COUNT_SIZE, ((numblocks << 16) | blocksize) as u32);

            // In order to reset the interrupt flags, they must be set to 1 (not 0), so writing it to itself will do that
            EMMC1.set(registers::INTERRUPT_FLAGS, EMMC1.get(registers::INTERRUPT_FLAGS));

            //printkln!("mmc: sending command {:?} {:x}", cmd, offset);
            EMMC1.set(registers::ARG1, offset as u32);
            EMMC1.set(registers::ARG2, (offset >> 32) as u32);
            EMMC1.set(registers::COMMAND, command_code(cmd));
            let rw_flag = if readwrite == ReadWrite::Read { EMMC1_INT_READ_READY } else { EMMC1_INT_WRITE_READY };
            wait_until_set(registers::INTERRUPT_FLAGS, rw_flag | EMMC1_INT_ANY_ERROR)?;

            let flags = EMMC1.get(registers::INTERRUPT_FLAGS);
            if flags & EMMC1_INT_ANY_ERROR != 0 {
                //printkln!("mmc: error occurred: {:x}", flags);
                Err(KernelError::IOError)
            } else {
                Ok(())
            }
        }
    }

    fn read_data(data: &mut [u8]) -> Result<(), KernelError> {
        wait_until_set(registers::INTERRUPT_FLAGS, EMMC1_INT_READ_READY)?;

        for i in (0..data.len()).step_by(4) {
            let value = unsafe { EMMC1.get(registers::DATA) };

            let bytes = if data.len() - i < 4 { data.len() - i } else { 4 };
            for j in 0..bytes {
                data[i + j] = (value >> (j * 8)) as u8;
            }
        }

        Ok(())
    }

    fn write_data(data: &[u8]) -> Result<(), KernelError> {
        wait_until_set(registers::INTERRUPT_FLAGS, EMMC1_INT_WRITE_READY)?;

        for i in (0..data.len()).step_by(4) {
            let mut value = 0;
            let bytes = if data.len() - i < 4 { data.len() - i } else { 4 };
            for j in 0..bytes {
                value |= (data[i + j] as u32) << (j * 8);
            }

            unsafe { EMMC1.set(registers::DATA, value) };
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
        Command::ReadMultiple       => 0x12220032,
        Command::WriteSingle        => 0x18220000,
        Command::WriteMultiple      => 0x19220022,
        Command::AppCommand         => 0x37000000,
        Command::SendCID            => 0x02010000,
        Command::SendRelAddr        => 0x03020000,
        Command::CardSelect         => 0x07030000,
    }
}

fn wait_until_set(reg: usize, mask: u32) -> Result<(), KernelError> {
    for _ in 0..100000 {
        unsafe {
            if (EMMC1.get(reg) & mask) != 0 {
                return Ok(());
            }
        }
    }
    Err(KernelError::DeviceTimeout)
}

fn wait_until_clear(reg: usize, mask: u32) -> Result<(), KernelError> {
    for _ in 0..100000 {
        unsafe {
            if (EMMC1.get(reg) & mask) == 0 {
                return Ok(());
            }
        }
    }
    Err(KernelError::DeviceTimeout)
}

