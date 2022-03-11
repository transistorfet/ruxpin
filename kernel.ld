__KERNEL_START_ADDR = 0x80000;

MEMORY
{
    ram : ORIGIN = 0x80000, LENGTH = 0x100000
}

ENTRY(_start)
EXTERN(_start)
SECTIONS
{
    .text __KERNEL_START_ADDR : {
        *(.text._start)
        *(.text)
    } > ram
    .rodata : { *(.rodata) } > ram
    .got : { *(.got) } > ram
    .data : { *(.data) } > ram
    .bss : { *(.bss) } > ram

    __KERNEL_END_ADDR = .;
}

/*
__KERNEL_START_ADDR = 0x80000;

ENTRY(_start)
SECTIONS
{
    . = __KERNEL_START_ADDR;

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
*/
