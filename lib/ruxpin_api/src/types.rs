
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
pub enum Seek {
    FromStart,
    FromCurrent,
    FromEnd,
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

