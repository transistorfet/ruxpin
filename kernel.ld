__KERNEL_START_ADDR = 0x80000;

ENTRY(_start)
SECTIONS
{
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
