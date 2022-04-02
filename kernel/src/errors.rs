
#[allow(dead_code)]
#[derive(Debug)]
pub enum KernelError {
    // Memory Errors
    AddressAlreadyMapped,
    AddressUnmapped,
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
    NotFile,
    NotDirectory,
    NoSuchFilesystem,
}

