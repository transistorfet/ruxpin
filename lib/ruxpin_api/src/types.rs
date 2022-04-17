
pub type UserID = u16;
pub type GroupID = u16;
pub type InodeNum = u32;
pub type Pid = i32;


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
pub struct OpenFlags(pub u16);

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


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FileAccess(pub u16);

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

impl From<FileAccess> for u16 {
    fn from(source: FileAccess) -> Self {
        source.0
    }
}

impl From<u16> for FileAccess {
    fn from(source: u16) -> Self {
        FileAccess(source)
    }
}


#[derive(Copy, Clone, Debug)]
pub struct Timestamp(pub u64);

impl From<Timestamp> for u64 {
    fn from(source: Timestamp) -> Self {
        source.0
    }
}

impl From<Timestamp> for u32 {
    fn from(source: Timestamp) -> Self {
        source.0 as u32
    }
}

impl From<u32> for Timestamp {
    fn from(source: u32) -> Self {
        Timestamp(source as u64)
    }
}


#[derive(Copy, Clone)]
pub struct FileDesc(pub usize);

impl FileDesc {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

pub struct DirEntry {
    pub inode: InodeNum,
    pub name_len: u8,
    pub name: [u8; 256],
}

impl DirEntry {
    pub fn new_empty() -> Self {
        Self {
            inode: 0,
            name_len: 0,
            name: [0; 256],
        }
    }

    pub fn new(inode: InodeNum, name: &[u8]) -> Self {
        let mut entry = Self::new_empty();
        entry.inode = inode;
        entry.copy_into(name);
        entry
    }

    pub fn copy_into(&mut self, source: &[u8]) {
        let name_len = if source.len() < 256 { source.len() } else { 256 };
        self.name[..name_len].copy_from_slice(&source[..name_len]);
        self.name_len = name_len as u8;
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(&self.name[..self.name_len as usize])
        }
    }
}


#[repr(usize)]
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ApiError {
    // Memory Errors
    AddressAlreadyMapped        = 1,
    AddressUnmapped             = 2,
    AddressMisaligned           = 3,
    UnexpectedGranualeSize      = 4,
    CorruptTranslationTable     = 5,
    OutOfMemory                 = 6,
    NoSegmentFound              = 7,

    // Device Errors
    OperationNotPermitted       = 8,
    DeviceTimeout               = 9,
    IOError                     = 10,
    InvalidIrq                  = 11,

    // File System Errors
    FileNotOpen                 = 12,
    FileNotFound                = 13,
    NoSuchDevice                = 14,
    NotAFile                    = 15,
    NotADirectory               = 16,
    IsADirectory                = 17,
    NoSuchFilesystem            = 18,
    BadFileNumber               = 19,
    TooManyFilesOpen            = 20,
    InvalidSuperblock           = 21,
    InvalidInode                = 22,
    IncompatibleFeatures        = 23,
    FileSizeTooLarge            = 24,
    OutOfDiskSpace              = 25,
    ReadOnlyFilesystem          = 26,

    NotExecutable               = 27,
    InvalidArgument             = 28,
    InvalidSegmentType          = 29,

    UnknownError                = 30,
}

impl From<usize> for ApiError {
    fn from(source: usize) -> Self {
        match source {
             1 => ApiError::AddressAlreadyMapped,
             2 => ApiError::AddressUnmapped,
             3 => ApiError::AddressMisaligned,
             4 => ApiError::UnexpectedGranualeSize,
             5 => ApiError::CorruptTranslationTable,
             6 => ApiError::OutOfMemory,
             7 => ApiError::NoSegmentFound,

             8 => ApiError::OperationNotPermitted,
             9 => ApiError::DeviceTimeout,
            10 => ApiError::IOError,
            11 => ApiError::InvalidIrq,

            12 => ApiError::FileNotOpen,
            13 => ApiError::FileNotFound,
            14 => ApiError::NoSuchDevice,
            15 => ApiError::NotAFile,
            16 => ApiError::NotADirectory,
            17 => ApiError::IsADirectory,
            18 => ApiError::NoSuchFilesystem,
            19 => ApiError::BadFileNumber,
            20 => ApiError::TooManyFilesOpen,
            21 => ApiError::InvalidSuperblock,
            22 => ApiError::InvalidInode,
            23 => ApiError::IncompatibleFeatures,
            24 => ApiError::FileSizeTooLarge,
            25 => ApiError::OutOfDiskSpace,
            26 => ApiError::ReadOnlyFilesystem,

            27 => ApiError::NotExecutable,
            28 => ApiError::InvalidArgument,
            29 => ApiError::InvalidSegmentType,

            _ => ApiError::UnknownError,
        }
    }
}

