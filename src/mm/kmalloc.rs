
use core::ptr;
use core::mem;

struct Block {
    size: usize,
    next: *mut Block,
}

struct Heap {
    free_blocks: *mut Block,
}


static mut MAIN_HEAP: Heap = Heap { free_blocks: ptr::null_mut() };


pub fn init_kernel_heap(addr: *mut i8, size: usize) {
    let mut space: *mut Block = addr.cast();

    unsafe {
        (*space).size = size;
        (*space).next = ptr::null_mut();

        MAIN_HEAP.free_blocks = space;
    }
}

pub unsafe fn kmalloc(mut size: usize) -> *mut i8 {
    let mut nextfree: *mut Block;
    let mut prev: *mut Block = ptr::null_mut();
    let mut cur: *mut Block = MAIN_HEAP.free_blocks;

    // Align the size to 4 bytes
    size += (4 - (size & 0x3)) & 0x3;
    let block_size = size + mem::size_of::<Block>();

    while !cur.is_null() {
        if (*cur).size >= block_size {
            // If the block can be split with enough room for another block struct and more than 8 bytes left over, then split it
            if (*cur).size >= block_size + mem::size_of::<Block>() + 8 {
                nextfree = cur.cast::<i8>().offset(block_size as isize).cast();
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
                MAIN_HEAP.free_blocks = nextfree;
            }

            return cur.offset(1).cast();
        }

        prev = cur;
        cur = (*cur).next;
    }
    // Out Of Memory
    panic!("Kernel out of memory!  Halting...\n");
}

pub unsafe fn kmfree(ptr: *mut i8) {
    let mut prev: *mut Block = ptr::null_mut();
    let mut block: *mut Block = ptr.cast::<Block>().offset(-1);
    let mut cur: *mut Block = MAIN_HEAP.free_blocks;

    while cur != ptr::null_mut() {
        if (*cur).next == block {
            panic!("Double free detected at {:x}! Halting...\n", cur as usize);
        }

        if cur.cast::<i8>().offset((*cur).size as isize).cast() == block {
            // Merge the free'd block with the previous block
            (*cur).size += (*block).size;

            // If this block is adjacent to the next free block, then merge them
            if cur.cast::<i8>().offset((*cur).size as isize).cast() == (*cur).next {
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
                MAIN_HEAP.free_blocks = block;
            }
            (*block).next = cur;

            // If this block is adjacent to the next free block, then merge them
            if block.cast::<i8>().offset((*block).size as isize).cast() == cur {
                (*block).size += (*cur).size;
                (*block).next = (*cur).next;
            }
            return;
        }

        prev = cur;
        cur = (*cur).next;
    }
}

/*
void print_free()
{
    printk_safe("free list:\n");
    for (struct block *cur = MAIN_HEAP.free_blocks; cur; cur = cur->next) {
        printk_safe("%x: %x\n", cur, cur->size);
    }
}
*/

