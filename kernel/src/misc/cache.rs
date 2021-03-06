
use core::ops::Deref;
use core::fmt::Debug;
use core::ptr::NonNull;
use core::marker::PhantomData;
use core::sync::atomic::{self, AtomicUsize, Ordering};

use alloc::vec::Vec;

use crate::debug;
use crate::misc::linkedlist::{UnownedLinkedList, UnownedLinkedListNode};


pub struct Cache<K, T> {
    max_size: usize,
    items: Vec<UnownedLinkedListNode<CacheArcInner<K, T>>>,
    order: UnownedLinkedList<CacheArcInner<K, T>>,
}

pub struct CacheArc<K, T> {
    ptr: NonNull<CacheArcInner<K, T>>,
    _marker: PhantomData<T>,
}

pub struct CacheArcInner<K, T> {
    refcount: AtomicUsize,
    key: K,
    data: T,
}

impl<K: Copy + PartialEq, T> Cache<K, T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            items: Vec::with_capacity(max_size),
            order: UnownedLinkedList::new(),
        }
    }

    pub fn clear<S, E>(&mut self, store: S) -> Result<(), E>
    where
        S: Fn(K, &T) -> Result<(), E>
    {
        for i in 0..self.items.len() {
            if self.items[i].refcount.load(Ordering::Relaxed) != 0 {
                //return Err(());
                panic!("cache: attempting to clear a cache with items still borrowed");
            }
            store(self.items[i].key, &self.items[i].data)?;
        }

        *self = Cache::new(self.max_size);
        Ok(())
    }

    pub fn commit<S, E>(&mut self, store: S) -> Result<(), E>
    where
        S: Fn(K, &T) -> Result<(), E>
    {
        for i in 0..self.items.len() {
            store(self.items[i].key, &self.items[i].data)?;
        }
        Ok(())
    }

    pub fn get<L, S, E>(&mut self, key: K, load: L, store: S) -> Result<CacheArc<K, T>, E>
    where
        L: FnOnce() -> Result<T, E>,
        S: Fn(K, &T) -> Result<(), E>
    {
        // Search the list for the matching object
        let mut iter = self.order.iter();
        while let Some(mut node) = iter.next() {
            if unsafe { node.get().key } == key {
                unsafe {
                    self.order.remove_node(node);
                    self.order.insert_head(node);
                }
                //debug!("cache: returning existing");
                return Ok(unsafe { node.get_mut() }.wrap_inner());
            }
        }

        self.insert(key, load, store)
    }

    pub fn insert<L, S, E>(&mut self, key: K, load: L, store: S) -> Result<CacheArc<K, T>, E>
    where
        L: FnOnce() -> Result<T, E>,
        S: Fn(K, &T) -> Result<(), E>
    {
        // If not every cache entry is in use, then allocate a new one and fetch the object
        if self.items.len() < self.max_size {
            self.items.push(UnownedLinkedListNode::new(CacheArcInner::new(key, load()?)));
            let i = self.items.len() - 1;
            unsafe {
                self.order.insert_head(self.items[i].as_node_ptr());
            }
            //debug!("cache: returning new");
            return Ok(self.items[i].wrap_inner());
        }

        // If the cache is full, then find the last entry in the list that has no references and recycle it
        let mut iter = self.order.iter_rev();
        while let Some(mut node) = iter.next() {
            if unsafe { node.get() }.refcount.load(Ordering::Relaxed) == 0 {
                store(key, &mut unsafe { node.get_mut() }.data)?;
                unsafe { node.get_mut() }.data = load()?;
                unsafe {
                    self.order.remove_node(node);
                    self.order.insert_head(node);
                }
                //debug!("cache: recycling old");
                return Ok(unsafe { node.get_mut() }.wrap_inner());
            }
        }

        panic!("Out of Cache");
    }
}

impl<K, T: Debug> Cache<K, T> {
    pub fn print(&mut self) {
        let mut i = 0;
        let mut iter = self.order.iter();
        debug!("Cache contents:");
        while let Some(node) = iter.next() {
            let item = unsafe { node.get() };
            debug!("{}: {:?}", i, item.data);
            i += 1;
        }
    }
}


impl<K, T> CacheArc<K, T> {
    fn from_inner(inner: NonNull<CacheArcInner<K, T>>) -> Self {
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

impl<K, T> Clone for CacheArc<K, T> {
    fn clone(&self) -> Self {
        CacheArc::from_inner(self.ptr)
    }
}

impl<K, T> Deref for CacheArc<K, T> {
    type Target = T;

    fn deref(&self) -> &T {
        let inner = unsafe { self.ptr.as_ref() };
        &inner.data
    }
}

impl<K, T> Drop for CacheArc<K, T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        // TODO I have no idea if this is right.  I don't want to decrement the count if it's already 0
        if inner.refcount.load(Ordering::Acquire) != 0 {
            inner.refcount.fetch_sub(1, Ordering::Acquire);
        }
        atomic::fence(Ordering::Release);
        // Don't need to drop inner because it's stored in the Vec in Cache<T>
    }
}

unsafe impl<K: Sync + Send, T: Sync + Send> Send for CacheArc<K, T> {}
unsafe impl<K: Sync + Send, T: Sync + Send> Sync for CacheArc<K, T> {}


impl<K, T> CacheArcInner<K, T> {
    fn new(key: K, data: T) -> Self {
        Self {
            refcount: AtomicUsize::new(0),
            key,
            data,
        }
    }

    fn wrap_inner(&mut self) -> CacheArc<K, T> {
        CacheArc::from_inner(NonNull::new(self.as_ptr()).unwrap())
    }

    fn as_ptr(&mut self) -> *mut CacheArcInner<K, T> {
        self as *mut CacheArcInner<K, T>
    }
}

