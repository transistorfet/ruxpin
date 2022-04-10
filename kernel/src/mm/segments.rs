
use crate::arch::types::{VirtualAddress, PhysicalAddress};


pub struct Segment {
    start: VirtualAddress,
    end: VirtualAddress,
    //ops for getting pages
}

impl Segment {
    pub fn new(start: VirtualAddress, end: VirtualAddress) -> Self {
        Self {
            start,
            end,
        }
    }

    pub fn match_range(&self, addr: VirtualAddress) -> bool {
        addr >= self.start && addr <= self.end
    }
}

