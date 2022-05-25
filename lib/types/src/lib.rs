#![no_std]

pub type UserID = u16;
pub type GroupID = u16;
pub type InodeNum = u32;
pub type Pid = i32;
pub type Tid = i32;


#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Seek {
    FromStart,
    FromCurrent,
    FromEnd,
}


pub type DriverID = u8;
pub type MinorDeviceID = u8;
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct DeviceID(pub DriverID, pub MinorDeviceID);


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
    pub const FileTypeMask: FileAccess          = FileAccess(0o170000);

    pub const Socket: FileAccess                = FileAccess(0o140000);
    pub const SymbolicLink: FileAccess          = FileAccess(0o120000);
    pub const Regular: FileAccess               = FileAccess(0o100000);

    pub const BlockDevice: FileAccess           = FileAccess(0o060000);
    pub const Directory: FileAccess             = FileAccess(0o040000);
    pub const CharDevice: FileAccess            = FileAccess(0o020000);
    pub const Fifo: FileAccess                  = FileAccess(0o010000);

    pub const SUID: FileAccess                  = FileAccess(0o004000);
    pub const SGID: FileAccess                  = FileAccess(0o002000);
    pub const StickBit: FileAccess              = FileAccess(0o001000);

    pub const OwnerRead: FileAccess             = FileAccess(0o000400);
    pub const OwnerWrite: FileAccess            = FileAccess(0o000200);
    pub const OwnerExec: FileAccess             = FileAccess(0o000100);

    pub const GroupRead: FileAccess             = FileAccess(0o000040);
    pub const GroupWrite: FileAccess            = FileAccess(0o000020);
    pub const GroupExec: FileAccess             = FileAccess(0o000010);

    pub const EveryoneRead: FileAccess          = FileAccess(0o000004);
    pub const EveryoneWrite: FileAccess         = FileAccess(0o000002);
    pub const EveryoneExec: FileAccess          = FileAccess(0o000001);

    pub const Read: FileAccess                  = FileAccess(0o000004);
    pub const Write: FileAccess                 = FileAccess(0o000002);
    pub const Exec: FileAccess                  = FileAccess(0o000001);

    pub const DefaultFile: FileAccess           = FileAccess(0o000644);
    pub const DefaultDir: FileAccess            = FileAccess(0o040755);
    pub const DefaultReadOnlyFile: FileAccess   = FileAccess(0o000444);

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
        let required_rwx = required_access.0 & 0o7;
        ((self.0 >> 6) & 0o7 & required_rwx) == required_rwx
    }

    pub fn require_everyone(self, required_access: Self) -> bool {
        let required_rwx = required_access.0 & 0o7;
        (self.0 & 0o7 & required_rwx) == required_rwx
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

const DIR_ENTRY_MAX_LEN: usize = 256;

pub struct DirEntry {
    pub inode: InodeNum,
    pub name_len: u8,
    pub name: [u8; DIR_ENTRY_MAX_LEN],
}

impl DirEntry {
    pub fn new_empty() -> Self {
        Self {
            inode: 0,
            name_len: 0,
            name: [0; DIR_ENTRY_MAX_LEN],
        }
    }

    pub fn new(inode: InodeNum, name: &[u8]) -> Self {
        let mut entry = Self::new_empty();
        entry.inode = inode;
        entry.copy_into(name);
        entry
    }

    pub fn copy_into(&mut self, source: &[u8]) {
        let name_len = if source.len() < DIR_ENTRY_MAX_LEN { source.len() } else { DIR_ENTRY_MAX_LEN };
        self.name[..name_len].copy_from_slice(&source[..name_len]);
        self.name_len = name_len as u8;
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(&self.name[..self.name_len as usize])
        }
    }

    pub unsafe fn set_len(&mut self, len: usize) {
        self.name_len = if len < DIR_ENTRY_MAX_LEN { len } else { DIR_ENTRY_MAX_LEN } as u8;
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
    MemoryPermissionDenied      = 8,

    // Device Errors
    NoSuchDevice                = 9,
    OperationNotPermitted       = 10,
    DeviceTimeout               = 11,
    IOError                     = 12,
    InvalidIrq                  = 13,

    // File System Errors
    FileNotOpen                 = 14,
    FileNotFound                = 15,
    NotAFile                    = 16,
    NotADirectory               = 17,
    IsADirectory                = 18,
    NoSuchFilesystem            = 19,
    BadFileNumber               = 20,
    TooManyFilesOpen            = 21,
    InvalidSuperblock           = 22,
    InvalidInode                = 23,
    IncompatibleFeatures        = 24,
    FileSizeTooLarge            = 25,
    OutOfDiskSpace              = 26,
    ReadOnlyFilesystem          = 27,
    AccessDenied                = 28,
    DirectoryNotEmpty           = 29,

    NoSuchTask                  = 30,
    NotExecutable               = 31,
    InvalidArgument             = 32,
    InvalidSegmentType          = 33,
    BadSystemCall               = 34,
    NotExited                   = 35,

    UnknownError                = 9999,
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
             8 => ApiError::MemoryPermissionDenied,

             9 => ApiError::NoSuchDevice,
            10 => ApiError::OperationNotPermitted,
            11 => ApiError::DeviceTimeout,
            12 => ApiError::IOError,
            13 => ApiError::InvalidIrq,

            14 => ApiError::FileNotOpen,
            15 => ApiError::FileNotFound,
            16 => ApiError::NotAFile,
            17 => ApiError::NotADirectory,
            18 => ApiError::IsADirectory,
            19 => ApiError::NoSuchFilesystem,
            20 => ApiError::BadFileNumber,
            21 => ApiError::TooManyFilesOpen,
            22 => ApiError::InvalidSuperblock,
            23 => ApiError::InvalidInode,
            24 => ApiError::IncompatibleFeatures,
            25 => ApiError::FileSizeTooLarge,
            26 => ApiError::OutOfDiskSpace,
            27 => ApiError::ReadOnlyFilesystem,
            28 => ApiError::AccessDenied,
            29 => ApiError::DirectoryNotEmpty,

            30 => ApiError::NoSuchTask,
            31 => ApiError::NotExecutable,
            32 => ApiError::InvalidArgument,
            33 => ApiError::InvalidSegmentType,
            34 => ApiError::BadSystemCall,
            35 => ApiError::NotExited,

            _ => ApiError::UnknownError,
        }
    }
}

