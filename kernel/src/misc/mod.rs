
pub mod cache;
pub mod bitmap;
pub mod strarray;
pub use self::strarray::StrArray;

pub mod deviceio;

pub mod linkedlist;


pub fn ceiling_div(size: usize, units: usize) -> usize {
    (size / units) + (size % units != 0) as usize
}

