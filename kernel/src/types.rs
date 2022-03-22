
use crate::errors::KernelError;

pub trait BlockDriver {
    fn init(&self) -> Result<(), KernelError>;
    fn read(&self, buffer: &mut [u8], offset: usize) -> Result<(), KernelError>;
    fn write(&self, buffer: &[u8], offset: usize) -> Result<(), KernelError>;
}

