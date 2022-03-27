
use crate::errors::KernelError;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum FileMode {
    Read,
    Write,
    ReadWrite
}

pub trait BlockDriver {
    fn init(&self) -> Result<(), KernelError>;
    fn read(&self, buffer: &mut [u8], offset: usize) -> Result<(), KernelError>;
    fn write(&self, buffer: &[u8], offset: usize) -> Result<(), KernelError>;
}

pub trait CharDriver {
    fn init(&mut self) -> Result<(), KernelError>;
    fn open(&mut self, mode: FileMode) -> Result<(), KernelError>;
    fn close(&mut self) -> Result<(), KernelError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, KernelError>;
    fn write(&mut self, buffer: &[u8]) -> Result<usize, KernelError>;
}

