
pub type UserID = u16;
pub type GroupID = u16;
pub type FileNum = usize;
pub type InodeNum = usize;


#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Seek {
    FromStart,
    FromCurrent,
    FromEnd,
}


pub type DriverID = u8;
pub type SubDeviceID = u8;
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct DeviceID(pub DriverID, pub SubDeviceID);


#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OpenFlags(u16);

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
impl OpenFlags {
    pub const ReadOnly: OpenFlags   = OpenFlags(0o0000);
    pub const WriteOnly: OpenFlags  = OpenFlags(0o0001);
    pub const ReadWrite: OpenFlags  = OpenFlags(0o0002);
    pub const Create: OpenFlags     = OpenFlags(0o0100);
    pub const Truncate: OpenFlags   = OpenFlags(0o1000);
    pub const Append: OpenFlags     = OpenFlags(0o2000);
    pub const NonBlock: OpenFlags   = OpenFlags(0o4000);

    pub fn and(self, flag: Self) -> Self {
        OpenFlags(self.0 | flag.0)
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

