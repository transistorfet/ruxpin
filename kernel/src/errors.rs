
use ruxpin_api::types::ApiError;

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
    MemoryPermissionDenied,

    // Device Errors
    NoSuchDevice,
    OperationNotPermitted,
    DeviceTimeout,
    IOError,
    InvalidIrq,

    // File System Errors
    FileNotOpen,
    FileNotFound,
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

    // Task Errors
    NoSuchTask,
    NotExecutable,
    InvalidArgument,
    InvalidSegmentType,
    BadSystemCall,
    NotExited,

    SuspendProcess,
}

impl From<KernelError> for ApiError {
    fn from(source: KernelError) -> ApiError {
        match source {
            KernelError::AddressAlreadyMapped           => ApiError::AddressAlreadyMapped,
            KernelError::AddressUnmapped                => ApiError::AddressUnmapped,
            KernelError::AddressMisaligned              => ApiError::AddressMisaligned,
            KernelError::UnexpectedGranualeSize         => ApiError::UnexpectedGranualeSize,
            KernelError::CorruptTranslationTable        => ApiError::CorruptTranslationTable,
            KernelError::OutOfMemory                    => ApiError::OutOfMemory,
            KernelError::NoSegmentFound                 => ApiError::NoSegmentFound,
            KernelError::MemoryPermissionDenied         => ApiError::MemoryPermissionDenied,

            KernelError::NoSuchDevice                   => ApiError::NoSuchDevice,
            KernelError::OperationNotPermitted          => ApiError::OperationNotPermitted,
            KernelError::DeviceTimeout                  => ApiError::DeviceTimeout,
            KernelError::IOError                        => ApiError::IOError,
            KernelError::InvalidIrq                     => ApiError::InvalidIrq,

            KernelError::FileNotOpen                    => ApiError::FileNotOpen,
            KernelError::FileNotFound                   => ApiError::FileNotFound,
            KernelError::NotAFile                       => ApiError::NotAFile,
            KernelError::NotADirectory                  => ApiError::NotADirectory,
            KernelError::IsADirectory                   => ApiError::IsADirectory,
            KernelError::NoSuchFilesystem               => ApiError::NoSuchFilesystem,
            KernelError::BadFileNumber                  => ApiError::BadFileNumber,
            KernelError::TooManyFilesOpen               => ApiError::TooManyFilesOpen,
            KernelError::InvalidSuperblock              => ApiError::InvalidSuperblock,
            KernelError::InvalidInode                   => ApiError::InvalidInode,
            KernelError::IncompatibleFeatures           => ApiError::IncompatibleFeatures,
            KernelError::FileSizeTooLarge               => ApiError::FileSizeTooLarge,
            KernelError::OutOfDiskSpace                 => ApiError::OutOfDiskSpace,
            KernelError::ReadOnlyFilesystem             => ApiError::ReadOnlyFilesystem,

            KernelError::NoSuchTask                     => ApiError::NoSuchTask,
            KernelError::NotExecutable                  => ApiError::NotExecutable,
            KernelError::InvalidArgument                => ApiError::InvalidArgument,
            KernelError::InvalidSegmentType             => ApiError::InvalidSegmentType,
            KernelError::BadSystemCall                  => ApiError::BadSystemCall,
            KernelError::NotExited                      => ApiError::NotExited,

            _ => ApiError::UnknownError,
        }
    }
}

