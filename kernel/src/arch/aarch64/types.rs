
use core::fmt;

use crate::misc::align_up;

//extern "C" {
//    static __KERNEL_VIRTUAL_BASE_ADDR: u64;
//}

#[inline(always)]
const fn kernel_virtual_base_addr() -> u64 {
    0xffff_0000_0000_0000
}


#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct PhysicalAddress(u64);

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct VirtualAddress(u64);

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct KernelVirtualAddress(u64);



impl From<u64> for PhysicalAddress {
    fn from(addr: u64) -> Self {
        if addr & kernel_virtual_base_addr() != 0 {
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

    pub fn to_kernel_addr(self) -> KernelVirtualAddress {
        KernelVirtualAddress::from(self)
    }
}



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

    pub fn align_down(self, align: usize) -> Self  {
        Self(self.0 & !(align - 1) as u64)
    }

    pub fn align_up(self, align: usize) -> Self  {
        Self(align_up(self.0 as usize, align) as u64)
    }

    pub fn offset_from_align(self, align: usize) -> usize {
        (self.0 as usize) & (align - 1)
    }
}



impl From<PhysicalAddress> for KernelVirtualAddress {
    fn from(addr: PhysicalAddress) -> Self {
        Self(addr.0 | kernel_virtual_base_addr())
    }
}

impl KernelVirtualAddress {
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset as u64)
    }

    pub const fn new(addr: u64) -> Self {
        Self(addr | kernel_virtual_base_addr())
    }

    pub unsafe fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    pub unsafe fn as_mut<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

