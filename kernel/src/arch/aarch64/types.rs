
use core::fmt;


#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct VirtualAddress(u64);

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct PhysicalAddress(u64);


impl From<u64> for VirtualAddress {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl From<VirtualAddress> for u64 {
    fn from(vaddr: VirtualAddress) -> Self {
        vaddr.0
    }
}

impl From<VirtualAddress> for usize {
    fn from(vaddr: VirtualAddress) -> Self {
        vaddr.0 as usize
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "VirtualAddress({:#x})", self.0)
    }
}

impl VirtualAddress {
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset as u64)
    }

    pub unsafe fn as_ptr(self) -> *mut u8 {
        self.0 as *mut u8
    }
}


impl From<u64> for PhysicalAddress {
    fn from(addr: u64) -> Self {
        if addr & 0xffff_0000_0000_0000 != 0 {
            panic!("physical address is using kernel space: {:x}", addr);
        }
        Self(addr)
    }
}

impl From<PhysicalAddress> for u64 {
    fn from(paddr: PhysicalAddress) -> Self {
        paddr.0
    }
}

impl From<PhysicalAddress> for usize {
    fn from(paddr: PhysicalAddress) -> usize {
        paddr.0 as usize
    }
}


impl From<*mut u8> for PhysicalAddress {
    fn from(addr: *mut u8) -> Self {
        PhysicalAddress::from(addr as u64)
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "PhysicalAddress({:#x})", self.0)
    }
}

impl PhysicalAddress {
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset as u64)
    }

    pub unsafe fn as_ptr(self) -> *mut u8 {
        self.0 as *mut u8
    }
}

