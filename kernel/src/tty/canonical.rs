
use alloc::boxed::Box;

use ruxpin_api::syscalls::SyscallFunction;

use crate::proc::process;
use crate::errors::KernelError;
use crate::tasklets::schedule_tasklet;

use super::CharOperations;


const INPUT_SIZE: usize = 4096;

pub struct CanonicalReader {
    in_pos: usize,
    buffer: [u8; INPUT_SIZE],
    ready: bool,
}

impl CanonicalReader {
    pub fn new() -> Self {
        Self {
            in_pos: 0,
            buffer: [0; INPUT_SIZE],
            ready: false,
        }
    }

    pub fn process_char(&mut self, dev: &mut dyn CharOperations, ch: u8) -> Result<bool, KernelError> {
        // TODO this is a hack for now to stop input when a full line is buffered
        if self.ready {
            return Ok(true);
        }

        if ch < 0x20 {
            match ch as char {
                '\n' | '\r' => {
                    dev.write(b"\n")?;
                    //process::restart_blocked(SyscallFunction::Read);
                    //schedule_tasklet(Box::new(|| {
                    //    process::restart_blocked(SyscallFunction::Read);
                    //    Ok(())
                    //}));

                    self.ready = true;
                    //dev.write(b"entered")?;
                }
                _ => { },
            }
        } else {
            self.buffer[self.in_pos] = ch;
            self.in_pos += 1;
            dev.write(&[ch])?;
        }
        Ok(self.ready)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, KernelError> {
        if self.ready {
            let nbytes = if buffer.len() < self.in_pos { buffer.len() } else { self.in_pos };
            (&mut buffer[..nbytes]).copy_from_slice(&mut self.buffer[..nbytes]);
            self.ready = false;
            self.in_pos = 0;
            Ok(nbytes)
        } else {
            Ok(0)
        }
    }
}

