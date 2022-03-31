
pub type UserID = u16;
pub type GroupID = u16;
pub type FileNum = usize;
pub type DeviceNum = u16;
pub type InodeNum = usize;

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Seek {
    FromStart,
    FromCurrent,
    FromEnd,
}


#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FileFlags(u16);

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
impl FileFlags {
    pub const ReadOnly: FileFlags   = FileFlags(0o0000);
    pub const WriteOnly: FileFlags  = FileFlags(0o0001);
    pub const ReadWrite: FileFlags  = FileFlags(0o0002);
    pub const Create: FileFlags     = FileFlags(0o0100);
    pub const Truncate: FileFlags   = FileFlags(0o1000);
    pub const Append: FileFlags     = FileFlags(0o2000);
    pub const NonBlock: FileFlags   = FileFlags(0o4000);

    pub fn and(self, flag: Self) -> Self {
        FileFlags(self.0 | flag.0)
    }

    pub fn is_set(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}


#[derive(Copy, Clone, Debug)]
pub struct FileAccess(u16);

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
impl FileAccess {
    pub const Directory: FileAccess     = FileAccess(0o40000);

    pub const OwnerRead: FileAccess     = FileAccess(0o00400);
    pub const OwnerWrite: FileAccess    = FileAccess(0o00200);
    pub const OwnerExec: FileAccess     = FileAccess(0o00100);

    pub const DefaultFile: FileAccess   = FileAccess(0o00644);
    pub const DefaultDir: FileAccess    = FileAccess(0o40755);

    pub fn and(self, flag: Self) -> Self {
        FileAccess(self.0 | flag.0)
    }

    pub fn is_set(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn is_dir(self) -> bool {
        self.is_set(FileAccess::Directory)
    }
}

