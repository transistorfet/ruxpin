
use core::mem;
use core::slice;
use core::mem::MaybeUninit;

pub unsafe fn copy_struct<T>(dest: &mut T, source: &[u8]) {
    let data_len = mem::size_of::<T>();
    let buffer = slice::from_raw_parts_mut(dest as *mut T as *mut u8, data_len);
    buffer.copy_from_slice(&source[..data_len]);
}

pub unsafe fn read_struct<T>(source: &[u8]) -> T {
    let mut dest: MaybeUninit<T> = MaybeUninit::uninit();
    let data_len = mem::size_of::<T>();
    let buffer = slice::from_raw_parts_mut(dest.assume_init_mut() as *mut T as *mut u8, data_len);
    buffer.copy_from_slice(&source[0..data_len]);
    dest.assume_init()
}

pub unsafe fn cast_to_slice<T>(source: &[u8]) -> &[T] {
    slice::from_raw_parts(source.as_ptr() as *const T, source.len() / mem::size_of::<T>())
}

pub unsafe fn cast_to_slice_mut<T>(source: &mut [u8]) -> &mut [T] {
    slice::from_raw_parts_mut(source.as_mut_ptr() as *mut T, source.len() / mem::size_of::<T>())
}

