
#[allow(dead_code)]
#[derive(Debug)]
pub enum KernelError {
    AddressAlreadyMapped,
    AddressUnmapped,
    CorruptTranslationTable,

    DeviceTimeout,
    PermissionNotAllowed,
}

