
use core::ptr;
use core::mem;
use core::alloc::{GlobalAlloc, Layout};

use ruxpin_api::sbrk;


struct Block {
    size: usize,
    next: *mut Block,
}

struct Heap {
    last_increase: usize,
    free_blocks: *mut Block,
}

#[global_allocator]
static mut MAIN_HEAP: Heap = Heap {
    last_increase: 2048,
    free_blocks: ptr::null_mut()
};

#[alloc_error_handler]
fn out_of_memory(_: Layout) -> ! {
    panic!("user allocator: out of memory");
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // TODO add mutex??
        MAIN_HEAP.malloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // TODO add mutex??
        MAIN_HEAP.free(ptr);
    }
}

impl Heap {
    pub unsafe fn malloc(&mut self, mut size: usize) -> *mut u8 {
        let mut nextfree: *mut Block;
        let mut prev: *mut Block = ptr::null_mut();
        let mut cur: *mut Block = self.free_blocks;

        // Align the size to 8 bytes
        size += (8 - (size & 0x7)) & 0x7;
        let block_size = size + mem::size_of::<Block>();

        loop {
            while !cur.is_null() {
                if (*cur).size >= block_size {
                    // If the block can be split with enough room for another block struct and more than 8 bytes left over, then split it
                    if (*cur).size >= block_size + mem::size_of::<Block>() + 8 {
                        nextfree = cur.cast::<u8>().add(block_size).cast();
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

            // Double the last increase (and if it's not big enough to allocate the requested size, keep doubling until it can)
            //do {
            //    self.last_increase <<= 1;
            //} while (self.last_increase < block_size);
            self.last_increase *= 2;

            // Ask the kernel to increase the data segment
            cur = sbrk(0).unwrap() as *mut Block;
            if let Err(_) = sbrk(self.last_increase) {
                // Out Of Memory
                return ptr::null_mut();
            }

            // Add the new block to the free list
            (*cur).size = self.last_increase;
            if !prev.is_null() {
                (*prev).next = cur;
            } else {
                (*cur).next = self.free_blocks;
                self.free_blocks = cur;
            }
        }
    }

    pub unsafe fn free(&mut self, ptr: *mut u8) {
        let mut prev: *mut Block = ptr::null_mut();
        let mut block: *mut Block = ptr.cast::<Block>().offset(-1);
        let mut cur: *mut Block = self.free_blocks;

        while !cur.is_null() {
            if (*cur).next == block {
                panic!("Double free detected at {:x}! Halting...\n", cur as usize);
            }

            if cur.cast::<u8>().add((*cur).size).cast() == block {
                // Merge the free'd block with the previous block
                (*cur).size += (*block).size;

                // If this block is adjacent to the next free block, then merge them
                if cur.cast::<u8>().add((*cur).size).cast() == (*cur).next {
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
                if block.cast::<u8>().add((*block).size).cast() == cur {
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

