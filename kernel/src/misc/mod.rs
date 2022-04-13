
pub mod cache;
pub mod bitmap;
pub mod byteorder;
pub mod memory;
pub mod strarray;
pub use self::strarray::StrArray;

pub mod deviceio;

pub mod linkedlist;


pub fn ceiling_div(size: usize, units: usize) -> usize {
    (size / units) + (size % units != 0) as usize
}

pub fn align_up(length: usize, alignment: usize) -> usize {
    ceiling_div(length, alignment) * alignment
}

