
use core::ptr;

use crate::printkln;
use crate::errors::KernelError;

//use super::gpio;


#[repr(u64)]
enum EmmcHostRegister {
    Arg2        = EMMC1_BASE_ADDR + 0x00,
    BlockCount  = EMMC1_BASE_ADDR + 0x04,
}

const EMMC1_BASE_ADDR: u64 = 0x3F30_0000;

const EMMC1_ARG2: *mut u32              = (EMMC1_BASE_ADDR + 0x00) as *mut u32;
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
const EMMC1_HOST_CONTROL2: *mut u32     = (EMMC1_BASE_ADDR + 0x3C) as *mut u32;

const EMMC1_HC1_CLOCK_STABLE: u32       = 1 << 1;

const EMMC1_CMD_GO_IDLE: u32            = 0x00000000;   // CMD0
const EMMC1_CMD_SEND_IF_COND: u32       = 0x08020000;   // CMD8
const EMMC1_CMD_STOP_TRANSMISSION: u32  = 0x0C030000;   // CMD12
const EMMC1_CMD_SEND_OP_COND: u32       = 0x29020000;   // ACMD41
const EMMC1_CMD_READ_SINGLE: u32        = 0x11220010;   // CMD17
const EMMC1_CMD_APP_COMMAND: u32        = 0x37000000;   // CMD55

const EMMC1_CMD_ALL_SEND_CID: u32       = 0x02010000;
const EMMC1_CMD_SEND_REL_ADDR: u32      = 0x03020000;
const EMMC1_CMD_CARD_SELECT: u32        = 0x07030000;

const EMMC1_STA_COMMAND_INHIBIT: u32    = 1 << 1;

const EMMC1_INT_COMMAND_DONE: u32       = 1 << 0;
const EMMC1_INT_DATA_DONE: u32          = 1 << 1;
const EMMC1_INT_ANY_ERROR: u32          = 0x17F << 16;


pub struct EmmcDevice;

impl EmmcDevice {
    pub fn init() {
        EmmcHost::init().unwrap();

        EmmcHost::send_command(EMMC1_CMD_GO_IDLE, 0).unwrap();

        EmmcHost::send_command(EMMC1_CMD_SEND_IF_COND, 0x000001AA).unwrap();
        EmmcHost::send_command(EMMC1_CMD_APP_COMMAND, 0).unwrap();
        EmmcHost::send_command(EMMC1_CMD_SEND_OP_COND, 0x51ff8000).unwrap();


        EmmcHost::send_command(EMMC1_CMD_ALL_SEND_CID, 0).unwrap();
        let card = EmmcHost::send_command(EMMC1_CMD_SEND_REL_ADDR, 0).unwrap();
        EmmcHost::send_command(EMMC1_CMD_CARD_SELECT, card).unwrap();
    }

    pub fn read_sector(sector: usize, buffer: &mut [u8]) -> Result<(), KernelError> {
        EmmcHost::set_block_size(1, 512);
        EmmcHost::send_command(EMMC1_CMD_READ_SINGLE, sector as u32 * 512)?;

        EmmcHost::read_data(buffer)?;

        EmmcHost::send_command(EMMC1_CMD_STOP_TRANSMISSION, 0)?;

        Ok(())
    }
}

pub struct EmmcHost;

impl EmmcHost {
    fn init() -> Result<(), KernelError> {
        //gpio::enable_emmc1();

        unsafe {
            // Reset all host circuitry
            ptr::write_volatile(EMMC1_HOST_CONTROL0, 0);
            ptr::write_volatile(EMMC1_HOST_CONTROL1, 1 << 24);

            // Wait for reset to clear
            while (ptr::read_volatile(EMMC1_HOST_CONTROL1) & (1 << 24)) != 0 { }

            // Configure the clock
            ptr::write_volatile(EMMC1_HOST_CONTROL1, 0x000E_6805);
            wait_until_set(EMMC1_HOST_CONTROL1, EMMC1_HC1_CLOCK_STABLE).unwrap();

            ptr::write_volatile(EMMC1_INTERRUPT_ENABLE, 0xffff_ffff);
            ptr::write_volatile(EMMC1_INTERRUPT_MASK, 0xffff_ffff);
        }

        Ok(())
    }

    fn set_block_size(numblocks: usize, blocksize: usize) {
        unsafe {
            ptr::write_volatile(EMMC1_BLOCK_COUNT_SIZE, ((numblocks << 16) | blocksize) as u32);
        }
    }

    fn send_command(cmd: u32, arg1: u32) -> Result<u32, KernelError> {
        unsafe {
            wait_until_clear(EMMC1_STATUS, EMMC1_STA_COMMAND_INHIBIT)?;

            ptr::write_volatile(EMMC1_ARG1, arg1);
            ptr::write_volatile(EMMC1_COMMAND, cmd);

            // TODO this causes it to hang, but I'm not sure why.  It was done by the bare metal raspi C project
            //ptr::write_volatile(EMMC1_INTERRUPT_FLAGS, ptr::read_volatile(EMMC1_INTERRUPT_FLAGS));

            wait_until_set(EMMC1_INTERRUPT_FLAGS, EMMC1_INT_COMMAND_DONE | EMMC1_INT_ANY_ERROR)?;

            let flags = ptr::read_volatile(EMMC1_INTERRUPT_FLAGS);
            if flags & EMMC1_INT_ANY_ERROR != 0 {
                printkln!("Error occurred: {:x}", flags);
            }

            let r0 = ptr::read_volatile(EMMC1_RESPONSE0);
            let r1 = ptr::read_volatile(EMMC1_RESPONSE1);
            let r2 = ptr::read_volatile(EMMC1_RESPONSE2);
            let r3 = ptr::read_volatile(EMMC1_RESPONSE3);
            printkln!("Response: {:x} {:x} {:x} {:x}", r0, r1, r2, r3);
            Ok(r0)
        }
    }

    fn read_data(data: &mut [u8]) -> Result<(), KernelError> {
        for i in (0..data.len()).step_by(4) {
            let value = unsafe { ptr::read_volatile(EMMC1_DATA) };

            for j in 0..4 {
                data[i + j] = (value >> (j * 8)) as u8;
            }
        }

        Ok(())
    }
}

fn wait_until_set(reg: *const u32, mask: u32) -> Result<(), KernelError> {
    for i in 0..1000 {
        unsafe {
            if (ptr::read_volatile(reg) & mask) != 0 {
                return Ok(());
            }
        }
    }
    Err(KernelError::DeviceTimeout)
}

fn wait_until_clear(reg: *const u32, mask: u32) -> Result<(), KernelError> {
    for i in 0..1000 {
        unsafe {
            if (ptr::read_volatile(reg) & mask) == 0 {
                return Ok(());
            }
        }
    }
    Err(KernelError::DeviceTimeout)
}

