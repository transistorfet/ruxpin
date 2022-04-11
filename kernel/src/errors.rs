
#[repr(usize)]
#[allow(dead_code)]
#[derive(Debug)]
pub enum KernelError {
    // Memory Errors
    AddressAlreadyMapped,
    AddressUnmapped,
    AddressMisaligned,
    UnexpectedGranualeSize,
    CorruptTranslationTable,
    OutOfMemory,

    // Device Errors
    OperationNotPermitted,
    DeviceTimeout,
    IOError,

    // File System Errors
    FileNotOpen,
    FileNotFound,
    NoSuchDevice,
    NotAFile,
    NotADirectory,
    IsADirectory,
    NoSuchFilesystem,
    BadFileNumber,
    TooManyFilesOpen,
    InvalidSuperblock,
    InvalidInode,
    IncompatibleFeatures,

    NotExecutable,
}

