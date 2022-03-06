__KERNEL_START_ADDR = 0x80000;

ENTRY(_start)
EXTERN(_start)
SECTIONS
{
    .text __KERNEL_START_ADDR : {
        *(.text._start)
        *(.text)
    }
    .rodata : { *(.rodata) }
    .got : { *(.got) }
    .data : { *(.data) }
    .bss : { *(.bss) }

    __KERNEL_END_ADDR = .;
}
