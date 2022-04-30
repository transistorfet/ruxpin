
use crate::errors::KernelError;

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

        if ch >= 0x20 && ch <= 0x7E {
            self.buffer[self.in_pos] = ch;
            self.in_pos += 1;
            dev.write(&[ch])?;
        } else {

            match ch as char {
                '\n' | '\r' => {
                    dev.write(b"\n")?;
                    self.ready = true;
                },
                '\x08' | '\x7f' => {
                    if self.in_pos > 0 {
                        dev.write(b"\x08 \x08")?;
                        self.in_pos -= 1;
                        self.buffer[self.in_pos] = ' ' as u8;
                    }
                },
                _ => {
                    // TODO this is for debugging
                    //let mut data = [0; 2];
                    //data[0] = (ch / 10) + 0x30;
                    //data[1] = (ch % 10) + 0x30;
                    //dev.write(&data)?;
                },
            }
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

