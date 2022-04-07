
pub mod cache;
pub mod bitmap;
pub mod strarray;
pub use self::strarray::StrArray;

pub mod deviceio;

pub mod linkedlist;


pub fn ceiling_div(size: usize, units: usize) -> usize {
    (size / units) + (size % units != 0) as usize
}

pub unsafe fn copy_struct<T>(dest: &mut T, source: &[u8]) {
    use core::mem;
    use core::slice;

    let data_len = mem::size_of::<T>();
    let buffer = slice::from_raw_parts_mut(dest as *mut T as *mut u8, data_len);
    buffer.copy_from_slice(&source[..data_len]);
}

