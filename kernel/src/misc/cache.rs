
use core::ops::Deref;

use core::ptr::NonNull;
use core::marker::PhantomData;
use core::sync::atomic::{self, AtomicUsize, Ordering};

use alloc::vec::Vec;

pub struct Cache<T> {
    max_size: usize,
    items: Vec<CacheArcInner<T>>,
}

pub struct CacheArc<T> {
    ptr: NonNull<CacheArcInner<T>>,
    _marker: PhantomData<T>,
}

pub struct CacheArcInner<T> {
    refcount: AtomicUsize,
    data: T,
}

impl<T> Cache<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            items: Vec::with_capacity(max_size),
        }
    }

    pub fn get<C, F>(&mut self, compare: C, fetch: F) -> CacheArc<T> where C: Fn(&T) -> bool, F: FnOnce() -> T {
        for item in self.items.iter_mut() {
            if compare(&item.data) {
                return item.wrap_inner();
            }
        }

        // TODO need a linked list or something to track age of last use
        for item in self.items.iter_mut() {
            if item.refcount.load(Ordering::Relaxed) == 0 {
                item.data = fetch();
                return item.wrap_inner();
            }
        }

        if self.items.len() < self.max_size {
            self.items.push(CacheArcInner::new(fetch()));
            let i = self.items.len() - 1;
            return self.items[i].wrap_inner();
        }

        panic!("Out of Cache");
    }
}


impl<T> CacheArc<T> {
    fn from_inner(inner: NonNull<CacheArcInner<T>>) -> Self {
        let inner_data = unsafe { inner.as_ref() };
        let count = inner_data.refcount.fetch_add(1, Ordering::Relaxed);

        if count == isize::MAX as usize {
            panic!("Too many references");
        }

        Self {
            ptr: inner,
            _marker: PhantomData,
        }
    }
}

impl<T: Clone> Clone for CacheArc<T> {
    fn clone(&self) -> Self {
        CacheArc::from_inner(self.ptr)
    }
}

impl<T> Deref for CacheArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let inner = unsafe { self.ptr.as_ref() };
        &inner.data
    }
}

impl<T> Drop for CacheArc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        if inner.refcount.load(Ordering::Acquire) != 0 {
            inner.refcount.fetch_sub(1, Ordering::Release);
        }
        atomic::fence(Ordering::Release);
        // Don't need to drop inner because it's stored in the Vec in Cache<T>
    }
}

unsafe impl<T: Sync + Send> Send for CacheArc<T> {}
unsafe impl<T: Sync + Send> Sync for CacheArc<T> {}


impl<T> CacheArcInner<T> {
    fn new(data: T) -> Self {
        Self {
            refcount: AtomicUsize::new(0),
            data,
        }
    }

    fn wrap_inner(&mut self) -> CacheArc<T> {
        CacheArc::from_inner(NonNull::new(self.as_ptr()).unwrap())
    }

    fn as_ptr(&mut self) -> *mut CacheArcInner<T> {
        self as *mut CacheArcInner<T>
    }
}

