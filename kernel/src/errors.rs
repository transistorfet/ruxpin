
#[repr(usize)]
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum KernelError {
    // Memory Errors
    AddressAlreadyMapped,
    AddressUnmapped,
    AddressMisaligned,
    UnexpectedGranualeSize,
    CorruptTranslationTable,
    OutOfMemory,
    NoSegmentFound,

    // Device Errors
    OperationNotPermitted,
    DeviceTimeout,
    IOError,
    InvalidIrq,

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
    FileSizeTooLarge,
    OutOfDiskSpace,
    ReadOnlyFilesystem,

    NotExecutable,
    InvalidArgument,
    InvalidSegmentType,

    SuspendProcess,
}

