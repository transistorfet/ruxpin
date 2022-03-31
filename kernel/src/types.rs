
use crate::errors::KernelError;

use ruxpin_api::types::FileFlags;

pub trait BlockDriver {
    fn init(&self) -> Result<(), KernelError>;
    fn read(&self, buffer: &mut [u8], offset: usize) -> Result<(), KernelError>;
    fn write(&self, buffer: &[u8], offset: usize) -> Result<(), KernelError>;
}

pub trait CharDriver {
    fn init(&mut self) -> Result<(), KernelError>;
    fn open(&mut self, mode: FileFlags) -> Result<(), KernelError>;
    fn close(&mut self) -> Result<(), KernelError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, KernelError>;
    fn write(&mut self, buffer: &[u8]) -> Result<usize, KernelError>;
}

