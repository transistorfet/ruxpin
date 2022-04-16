
use core::ptr::NonNull;
use core::ops::{Deref, DerefMut};


pub struct UnownedLinkedList<T> {
    head: Option<UnownedLinkedListRef<T>>,
    tail: Option<UnownedLinkedListRef<T>>,
}

pub struct UnownedLinkedListNode<T> {
    next: Option<UnownedLinkedListRef<T>>,
    prev: Option<UnownedLinkedListRef<T>>,
    data: T,
}

pub struct UnownedLinkedListRef<T>(NonNull<UnownedLinkedListNode<T>>);

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

    pub fn get_head(&mut self) -> Option<UnownedLinkedListRef<T>> {
        self.head
    }

    pub unsafe fn insert_head(&mut self, node: UnownedLinkedListRef<T>) {
        self.insert_after(node, None);
    }

    pub unsafe fn insert_tail(&mut self, node: UnownedLinkedListRef<T>) {
        self.insert_after(node, self.tail);
    }

    unsafe fn insert_after(&mut self, node: UnownedLinkedListRef<T>, after: Option<UnownedLinkedListRef<T>>) {
        if node.next().is_some() || node.prev().is_some() {
            panic!("attempting to re-add a node");
        }

	// If `after` is None then insert at the start of the list (ie. self.head)
	let tail = if after.is_some() {
            after.unwrap().next()
	} else {
	    self.head
        };

	// Connect the tail of the list to the node
	if tail.is_some() {
            tail.unwrap().set_prev(Some(node));
        } else {
            self.tail = Some(node);
        }
        node.set_next(tail);

	// Connect the list up to and including `after` to the node
        if after.is_some() {
            after.unwrap().set_next(Some(node));
        } else {
            self.head = Some(node);
        }
        node.set_prev(after);
    }

    pub unsafe fn remove_node(&mut self, node: UnownedLinkedListRef<T>) {
        if node.next().is_some() {
            node.next().unwrap().set_prev(node.prev());
        } else {
            self.tail = node.prev();
        }

        if node.prev().is_some() {
            node.prev().unwrap().set_next(node.next());
        } else {
            self.head = node.next();
        }

        node.set_next(None);
        node.set_prev(None);
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

    pub fn as_node_ptr(&mut self) -> UnownedLinkedListRef<T> {
        UnownedLinkedListRef(NonNull::new(self as *mut Self).unwrap())
    }

    pub fn next(&self) -> Option<UnownedLinkedListRef<T>> {
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



impl<T> Clone for UnownedLinkedListRef<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Copy for UnownedLinkedListRef<T> {}

impl<T> UnownedLinkedListRef<T> {
    pub unsafe fn get(&self) -> &T {
        &(*self.0.as_ptr()).data
    }

    pub unsafe fn get_mut(&mut self) -> &mut T {
        &mut (*self.0.as_ptr()).data
    }

    pub unsafe fn next(&self) -> Option<Self> {
        (*self.0.as_ptr()).next
    }

    pub unsafe fn prev(&self) -> Option<Self> {
        (*self.0.as_ptr()).prev
    }

    unsafe fn set_next(&self, node: Option<UnownedLinkedListRef<T>>) {
        (*self.0.as_ptr()).next = node;
    }

    unsafe fn set_prev(&self, node: Option<UnownedLinkedListRef<T>>) {
        (*self.0.as_ptr()).prev = node;
    }
}



pub struct UnownedLinkedListIter<T> {
    current: Option<UnownedLinkedListRef<T>>,
}

impl<T> UnownedLinkedListIter<T> {
    pub fn new(list: &UnownedLinkedList<T>) -> Self {
        Self {
            current: list.head,
        }
    }
}

impl<T> Iterator for UnownedLinkedListIter<T> {
    type Item = UnownedLinkedListRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        self.current = self.current.map(|node| unsafe { node.next() }).flatten();
        result
    }
}


pub struct UnownedLinkedListIterRev<T> {
    current: Option<UnownedLinkedListRef<T>>,
}

impl<T> UnownedLinkedListIterRev<T> {
    pub fn new(list: &UnownedLinkedList<T>) -> Self {
        Self {
            current: list.tail,
        }
    }
}

impl<T> Iterator for UnownedLinkedListIterRev<T> {
    type Item = UnownedLinkedListRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        self.current = self.current.map(|node| unsafe { node.prev() }).flatten();
        result
    }
}

