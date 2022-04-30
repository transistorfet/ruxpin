
pub mod bitmap;
pub mod byteorder;
pub mod cache;
pub mod memory;
pub mod strarray;
pub mod writer;

pub mod deviceio;

pub mod queue;
pub mod linkedlist;


pub fn ceiling_div(size: usize, units: usize) -> usize {
    (size / units) + (size % units != 0) as usize
}

pub fn align_up(length: usize, alignment: usize) -> usize {
    ceiling_div(length, alignment) * alignment
}

pub fn align_down(length: usize, alignment: usize) -> usize {
    length & !(alignment - 1)
}

