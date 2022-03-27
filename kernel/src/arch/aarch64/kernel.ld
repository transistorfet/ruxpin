__KERNEL_START_ADDR = 0x80000;
__KERNEL_VIRTUAL_BASE_ADDR = 0xffff000000000000;

ENTRY(_start)
SECTIONS
{
    . = __KERNEL_VIRTUAL_BASE_ADDR;
    . += __KERNEL_START_ADDR;

    .text : {
        *(.text._start)
        *(.text*)
    }
    .rodata : { *(.rodata*) }
    .got : { *(.got*) }
    .data : { *(.data*) }
    .bss : { *(.bss*) }

    __KERNEL_END_ADDR = .;
}

