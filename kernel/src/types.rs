
use crate::errors::KernelError;


pub type FileNumber = usize;
pub type UserID = u16;
pub type GroupID = u16;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum FileFlags {
    Read,
    Write,
    ReadWrite
}

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum FileAccess {
    Directory   = 0o40000,

    OwnerRead   = 0o00400,
    OwnerWrite  = 0o00200,
    OwnerExec   = 0o00100,

    DefaultFile = 0o00644,
    DefaultDir  = 0o40755,
}

impl FileAccess {
    pub fn is_dir(self) -> bool {
        (self as u16) & (FileAccess::Directory as u16) != 0
    }
}

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

