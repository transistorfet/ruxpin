
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
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub fn get_head(&mut self) -> Option<NonNull<UnownedLinkedListNode<T>>> {
        self.head
    }

    pub unsafe fn insert_head(&mut self, node: NonNull<UnownedLinkedListNode<T>>) {
        if (*node.as_ptr()).next.is_some() || (*node.as_ptr()).prev.is_some() {
            panic!("attempting to re-add a node");
        }

        if self.head.is_some() {
            (*self.head.as_mut().unwrap().as_ptr()).prev = Some(node);
        } else {
            self.tail = Some(node);
        }
        (*node.as_ptr()).next = self.head;
        self.head = Some(node);
    }

    pub unsafe fn insert_tail(&mut self, node: NonNull<UnownedLinkedListNode<T>>) {
        self.insert_after(node, self.tail);
    }

    unsafe fn insert_after(&mut self, node: NonNull<UnownedLinkedListNode<T>>, mut after: Option<NonNull<UnownedLinkedListNode<T>>>) {
	// If `after` is None then insert at the start of the list (ie. self.head)
	let mut tail = if after.is_some() {
	    (*after.as_mut().unwrap().as_ptr()).next
	} else {
	    self.head
        };

	// Connect the tail of the list to the node
	if tail.is_some() {
            (*tail.as_mut().unwrap().as_ptr()).prev = Some(node);
        } else {
            self.tail = Some(node);
        }
        (*node.as_ptr()).next = tail;

	// Connect the list up to and including `after` to the node
        if after.is_some() {
            (*after.as_mut().unwrap().as_ptr()).next = Some(node);
        } else {
            self.head = Some(node);
        }
        (*node.as_ptr()).prev = after;
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

        (*node.as_ptr()).next = None;
        (*node.as_ptr()).prev = None;
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

    pub fn as_node_ptr(&mut self) -> NonNull<Self> {
        NonNull::new(self as *mut Self).unwrap()
    }

    pub fn next(&self) -> Option<NonNull<Self>> {
        self.next
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

