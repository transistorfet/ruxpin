
pub type UserID = u16;
pub type GroupID = u16;
pub type InodeNum = u32;


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


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

    pub fn plus(self, flag: Self) -> Self {
        OpenFlags(self.0 | flag.0)
    }

    pub fn is_set(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn required_access(self) -> FileAccess {
        match OpenFlags(0o3 & self.0) {
            OpenFlags::ReadOnly => FileAccess::Read,
            OpenFlags::WriteOnly => FileAccess::Write,
            OpenFlags::ReadWrite => FileAccess::Read.plus(FileAccess::Write),
            _ => FileAccess::Read,
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FileAccess(u16);

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
impl FileAccess {
    pub const FileTypeMask: FileAccess  = FileAccess(0o170000);

    pub const Socket: FileAccess        = FileAccess(0o140000);
    pub const SymbolicLink: FileAccess  = FileAccess(0o120000);
    pub const Regular: FileAccess       = FileAccess(0o100000);

    pub const BlockDevice: FileAccess   = FileAccess(0o060000);
    pub const Directory: FileAccess     = FileAccess(0o040000);
    pub const CharDevice: FileAccess    = FileAccess(0o020000);
    pub const Fifo: FileAccess          = FileAccess(0o010000);

    pub const SUID: FileAccess          = FileAccess(0o004000);
    pub const SGID: FileAccess          = FileAccess(0o002000);
    pub const StickBit: FileAccess      = FileAccess(0o001000);

    pub const OwnerRead: FileAccess     = FileAccess(0o000400);
    pub const OwnerWrite: FileAccess    = FileAccess(0o000200);
    pub const OwnerExec: FileAccess     = FileAccess(0o000100);

    pub const GroupRead: FileAccess     = FileAccess(0o000040);
    pub const GroupWrite: FileAccess    = FileAccess(0o000020);
    pub const GroupExec: FileAccess     = FileAccess(0o000010);

    pub const EveryoneRead: FileAccess  = FileAccess(0o000004);
    pub const EveryoneWrite: FileAccess = FileAccess(0o000002);
    pub const EveryoneExec: FileAccess  = FileAccess(0o000001);

    pub const Read: FileAccess          = FileAccess(0o000004);
    pub const Write: FileAccess         = FileAccess(0o000002);
    pub const Exec: FileAccess          = FileAccess(0o000001);

    pub const DefaultFile: FileAccess   = FileAccess(0o000644);
    pub const DefaultDir: FileAccess    = FileAccess(0o040755);

    pub fn plus(self, flag: Self) -> Self {
        FileAccess(self.0 | flag.0)
    }

    pub fn is_set(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn is_dir(self) -> bool {
        (self.0 & FileAccess::FileTypeMask.0) == FileAccess::Directory.0
    }

    pub fn is_file(self) -> bool {
        (self.0 & FileAccess::FileTypeMask.0) == FileAccess::Regular.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn file_type(self) -> FileAccess {
        FileAccess(self.0 & FileAccess::FileTypeMask.0)
    }

    pub fn require_owner(self, required_access: Self) -> bool {
        ((self.0 >> 6) & 0o7 & required_access.0) == required_access.0
    }

    pub fn require_everyone(self, required_access: Self) -> bool {
        (self.0 & 0o7 & required_access.0) == required_access.0
    }
}

impl Into<FileAccess> for u16 {
    fn into(self) -> FileAccess {
        FileAccess(self)
    }
}


#[derive(Copy, Clone, Debug)]
pub struct Timestamp(pub u64);

impl Into<Timestamp> for u32 {
    fn into(self) -> Timestamp {
        Timestamp(self as u64)
    }
}


#[derive(Copy, Clone)]
pub struct FileDesc(pub usize);

impl FileDesc {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ApiError {
    SomethingWentWrong,
}

