
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use super::exceptions::{IrqFlags, enable_irq, disable_irq};


pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
}

pub struct Mutex<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(t: T) -> Mutex<T> {
        Mutex {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        unsafe {
            while !self.locked.try_change(true) {
                // TODO delay
            }
            MutexGuard { lock: self }
        }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            &*self.lock.data.get()
        }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            &mut *self.lock.data.get()
        }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.lock.locked.change(false);
        }
    }
}


pub struct AtomicBool {
    inner: UnsafeCell<bool>
}

impl AtomicBool {
    pub const fn new(value: bool) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    pub fn try_change(&self, value: bool) -> bool {
        unsafe {
            let flags = disable_irq();
            let result = if *self.inner.get() == value {
                false
            } else {
                *self.inner.get() = value;
                true
            };
            enable_irq(flags);
            result
        }
    }

    pub fn change(&self, value: bool) {
        unsafe {
            let flags = disable_irq();
            *self.inner.get() = value;
            enable_irq(flags);
        }
    }
}

unsafe impl Send for AtomicBool {}
unsafe impl Sync for AtomicBool {}

