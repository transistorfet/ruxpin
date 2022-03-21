
#[derive(Debug)]
pub enum KernelError {
    AddressAlreadyMapped,
    AddressUnmapped,
    CorruptTranslationTable,

    DeviceTimeout,
}

