
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use super::exceptions::{enable_irq, disable_irq};


pub struct SpinlockGuard<'a, T: ?Sized + 'a> {
    lock: &'a Spinlock<T>,
}

pub struct Spinlock<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>
}

unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(t: T) -> Spinlock<T> {
        Spinlock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        while !self.locked.try_change(true) {
            // TODO delay
        }
        SpinlockGuard { lock: self }
    }
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            &*self.lock.data.get()
        }
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            &mut *self.lock.data.get()
        }
    }
}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.locked.change(false);
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

