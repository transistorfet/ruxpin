
use core::ptr::NonNull;
use core::ops::{Deref, DerefMut};


pub struct UnownedLinkedList<T> {
    head: Option<NonNull<UnownedLinkedListNode<T>>>,
    tail: Option<NonNull<UnownedLinkedListNode<T>>>,
}

pub struct UnownedLinkedListNode<T> {
    next: Option<NonNull<UnownedLinkedListNode<T>>>,
    prev: Option<NonNull<UnownedLinkedListNode<T>>>,
    data: T,
}

unsafe impl<T: Sync + Send> Send for UnownedLinkedList<T> {}
unsafe impl<T: Sync + Send> Sync for UnownedLinkedList<T> {}
unsafe impl<T: Sync + Send> Send for UnownedLinkedListNode<T> {}
unsafe impl<T: Sync + Send> Sync for UnownedLinkedListNode<T> {}


impl<T> UnownedLinkedList<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub unsafe fn insert_head(&mut self, node: NonNull<UnownedLinkedListNode<T>>) {
        if self.head.is_some() {
            (*self.head.as_mut().unwrap().as_ptr()).prev = Some(node);
        } else {
            self.tail = Some(node);
        }
        (*node.as_ptr()).next = self.head;
        self.head = Some(node);
    }

    pub unsafe fn remove_node(&mut self, node: NonNull<UnownedLinkedListNode<T>>) {
        if (*node.as_ptr()).next.is_some() {
            (*(*node.as_ptr()).next.as_mut().unwrap().as_ptr()).prev = (*node.as_ptr()).prev;
        } else {
            self.tail = (*node.as_ptr()).prev;
        }

        if (*node.as_ptr()).prev.is_some() {
            (*(*node.as_ptr()).prev.as_mut().unwrap().as_ptr()).next = (*node.as_ptr()).next;
        } else {
            self.head = (*node.as_ptr()).next;
        }
    }

    pub fn iter(&self) -> UnownedLinkedListIter<T> {
        UnownedLinkedListIter::new(self)
    }

    pub fn iter_rev(&self) -> UnownedLinkedListIterRev<T> {
        UnownedLinkedListIterRev::new(self)
    }
}

impl<T> UnownedLinkedListNode<T> {
    pub fn new(data: T) -> Self {
        Self {
            next: None,
            prev: None,
            data,
        }
    }

    pub fn wrap_non_null(&mut self) -> NonNull<Self> {
        NonNull::new(self as *mut Self).unwrap()
    }
}

impl<T> Deref for UnownedLinkedListNode<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for UnownedLinkedListNode<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}



pub struct UnownedLinkedListIter<T> {
    current: Option<NonNull<UnownedLinkedListNode<T>>>,
}

impl<T> UnownedLinkedListIter<T> {
    pub fn new(list: &UnownedLinkedList<T>) -> Self {
        Self {
            current: list.head,
        }
    }
}

impl<T> Iterator for UnownedLinkedListIter<T> {
    type Item = NonNull<UnownedLinkedListNode<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        self.current = self.current.map(|node| unsafe { (*node.as_ptr()).next }).flatten();
        result
    }
}


pub struct UnownedLinkedListIterRev<T> {
    current: Option<NonNull<UnownedLinkedListNode<T>>>,
}

impl<T> UnownedLinkedListIterRev<T> {
    pub fn new(list: &UnownedLinkedList<T>) -> Self {
        Self {
            current: list.tail,
        }
    }
}

impl<T> Iterator for UnownedLinkedListIterRev<T> {
    type Item = NonNull<UnownedLinkedListNode<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        self.current = self.current.map(|node| unsafe { (*node.as_ptr()).prev }).flatten();
        result
    }
}

