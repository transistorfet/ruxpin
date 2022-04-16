
use alloc::sync::Arc;
use core::ops::{Deref, DerefMut};

use crate::sync::Spinlock;


pub type QueueNodeRef<T> = Arc<Spinlock<QueueNode<T>>>;

pub struct Queue<T> {
    max: Option<usize>,
    size: usize,
    head: Option<QueueNodeRef<T>>,
    tail: Option<QueueNodeRef<T>>,
}

pub struct QueueNode<T> {
    pub data: T,
    next: Option<QueueNodeRef<T>>,
    prev: Option<QueueNodeRef<T>>,
}

impl<T> Queue<T> {
    pub const fn new(max: Option<usize>) -> Self {

        Self {
            max,
            size: 0,
            head: None,
            tail: None,
        }
    }

    pub fn get_head(&self) -> Option<QueueNodeRef<T>> {
        self.head.clone()
    }

    pub fn get_tail(&self) -> Option<QueueNodeRef<T>> {
        self.tail.clone()
    }

    pub fn clear(&mut self) {
        while self.head.is_some() {
            let _ = self.remove_node(self.head.clone().unwrap());
        }
    }

    pub fn insert_head(&mut self, node: QueueNodeRef<T>) {
        self.insert_after(node, None);
    }

    pub fn insert_tail(&mut self, node: QueueNodeRef<T>) {
        self.insert_after(node, self.tail.clone());
    }

    pub fn insert_after(&mut self, node: QueueNodeRef<T>, mut after: Option<QueueNodeRef<T>>) {
        let mut locked_node = node.try_lock().unwrap();

        if locked_node.next.is_some() || locked_node.prev.is_some() {
            panic!("attempting to re-add a node");
        }

        // Remove the node if the queue has a limit
        if self.max.is_some() && self.size >= self.max.unwrap() {
            self.remove_node(self.tail.clone().unwrap());
        } else {
            self.size += 1;
        }

	// If `after` is None then insert at the start of the list (ie. self.head)
	let mut tail = if after.is_some() {
            after.as_mut().unwrap().lock().next.clone()
	} else {
	    self.head.clone()
        };

	// Connect the tail of the list to the node
	if tail.is_some() {
            tail.as_mut().unwrap().lock().prev = Some(node.clone());
        } else {
            self.tail = Some(node.clone());
        }
        locked_node.next = tail.clone();

	// Connect the list up to and including `after` to the node
        if after.is_some() {
            after.as_mut().unwrap().lock().next = Some(node.clone());
        } else {
            self.head = Some(node.clone());
        }
        locked_node.prev = after;
    }

    pub fn remove_node(&mut self, node: QueueNodeRef<T>) {
        let mut locked_node = node.lock();
        if locked_node.next.is_some() {
            locked_node.next.as_mut().unwrap().lock().prev = locked_node.prev.clone();
        } else {
            self.tail = locked_node.prev.clone();
        }

        if locked_node.prev.is_some() {
            locked_node.prev.as_mut().unwrap().lock().next = locked_node.next.clone();
        } else {
            self.head = locked_node.next.clone();
        }

        locked_node.next = None;
        locked_node.prev = None;
    }

    pub fn find<F>(&self, mut conditional: F) -> Option<QueueNodeRef<T>> where F: FnMut(&T) -> bool {
        let mut current = self.head.clone();

        while current.is_some() {
            current = {
                let locked_current = current.as_mut().unwrap().lock();
                if conditional(&locked_current.data) {
                    break;
                }
                locked_current.next.clone()
            }
        }
        current
    }

    pub fn foreach<F>(&self, mut f: F) where F: FnMut(&T) {
        let mut current = self.head.clone();

        while current.is_some() {
            current = {
                let locked_current = current.as_mut().unwrap().lock();
                f(&locked_current.data);
                locked_current.next.clone()
            }
        }
    }

    pub fn iter<'a>(&'a mut self) -> impl Iterator<Item=QueueNodeRef<T>> {
        QueueIterator {
            current: self.head.clone(),
        }
    }
}

impl<T> QueueNode<T> {
    pub fn new(data: T) -> QueueNodeRef<T> {
        Arc::new(Spinlock::new(Self {
            data,
            next: None,
            prev: None,
        }))
    }

    pub fn get<'a>(&'a self) -> &'a T {
        &self.data
    }

    pub fn get_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.data
    }
}

impl<T> Deref for QueueNode<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for QueueNode<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}


struct QueueIterator<T> {
    current: Option<QueueNodeRef<T>>,
}

impl<T> Iterator for QueueIterator<T> {
    type Item = QueueNodeRef<T>;

    fn next(&mut self) -> Option<QueueNodeRef<T>> {
        let result = self.current.clone();
        if self.current.is_some() {
            let next = self.current.as_mut().unwrap().lock().next.clone();
            self.current = next;
        }
        result
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

