
#[allow(dead_code)]
#[derive(Debug)]
pub enum KernelError {
    AddressAlreadyMapped,
    AddressUnmapped,
    UnexpectedGranualeSize,
    CorruptTranslationTable,

    DeviceTimeout,
    PermissionNotAllowed,

    FileNotOpen,
    IOError,
}

