
use core::ptr;
use core::mem;
use core::alloc::{GlobalAlloc, Layout};


#[global_allocator]
static mut MAIN_HEAP: Heap = Heap { free_blocks: ptr::null_mut() };

#[alloc_error_handler]
fn out_of_memory(_: Layout) -> ! {
    panic!("Out Of Memory");
}

struct Block {
    size: usize,
    next: *mut Block,
}

struct Heap {
    free_blocks: *mut Block,
}

impl Heap {
    pub unsafe fn init(&mut self, start: *mut u8, end: *mut u8) {
        let mut space: *mut Block = start.cast();

        (*space).size = end as usize - start as usize;
        (*space).next = ptr::null_mut();

        self.free_blocks = space;
    }

    pub unsafe fn malloc(&mut self, mut size: usize) -> *mut u8 {
        let mut nextfree: *mut Block;
        let mut prev: *mut Block = ptr::null_mut();
        let mut cur: *mut Block = self.free_blocks;

        // Align the size to 8 bytes
        size += (8 - (size & 0x7)) & 0x7;
        let block_size = size + mem::size_of::<Block>();

        while !cur.is_null() {
            if (*cur).size >= block_size {
                // If the block can be split with enough room for another block struct and more than 8 bytes left over, then split it
                if (*cur).size >= block_size + mem::size_of::<Block>() + 8 {
                    nextfree = cur.cast::<u8>().offset(block_size as isize).cast();
                    (*nextfree).size = (*cur).size - block_size;
                    (*cur).size = block_size;

                    (*nextfree).next = (*cur).next;

                } else {
                    nextfree = (*cur).next;
                }
                (*cur).next = ptr::null_mut();

                if !prev.is_null() {
                    (*prev).next = nextfree;
                } else {
                    self.free_blocks = nextfree;
                }

                return cur.offset(1).cast();
            }

            prev = cur;
            cur = (*cur).next;
        }
        // Out Of Memory
        panic!("Kernel out of memory!  Halting...\n");
    }

    pub unsafe fn free(&mut self, ptr: *mut u8) {
        let mut prev: *mut Block = ptr::null_mut();
        let mut block: *mut Block = ptr.cast::<Block>().offset(-1);
        let mut cur: *mut Block = self.free_blocks;

        while cur != ptr::null_mut() {
            if (*cur).next == block {
                panic!("Double free detected at {:x}! Halting...\n", cur as usize);
            }

            if cur.cast::<u8>().offset((*cur).size as isize).cast() == block {
                // Merge the free'd block with the previous block
                (*cur).size += (*block).size;

                // If this block is adjacent to the next free block, then merge them
                if cur.cast::<u8>().offset((*cur).size as isize).cast() == (*cur).next {
                    (*cur).size += (*(*cur).next).size;
                    (*cur).next = (*(*cur).next).next;
                }
                return;
            }

            if cur >= block {
                // Insert the free'd block into the list
                if !prev.is_null() {
                    (*prev).next = block;
                } else {
                    self.free_blocks = block;
                }
                (*block).next = cur;

                // If this block is adjacent to the next free block, then merge them
                if block.cast::<u8>().offset((*block).size as isize).cast() == cur {
                    (*block).size += (*cur).size;
                    (*block).next = (*cur).next;
                }
                return;
            }

            prev = cur;
            cur = (*cur).next;
        }
    }
}


pub unsafe fn init_kernel_heap(start: *mut u8, end: *mut u8) {
    MAIN_HEAP.init(start, end);
}

pub unsafe fn kmalloc(size: usize) -> *mut u8 {
    MAIN_HEAP.malloc(size)
}

pub unsafe fn kmfree(ptr: *mut u8) {
    MAIN_HEAP.free(ptr);
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // TODO add mutex??
        kmalloc(layout.size())
        //self.malloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // TODO add mutex??
        kmfree(ptr);
        //self.free(ptr);
    }
}

