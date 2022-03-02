_START_ADDR = 0x80000;

ENTRY(_start)
EXTERN(_start)
SECTIONS
{
    .text _START_ADDR : {
        *(.text._start)
        *(.text)
    }
    .rodata : { *(.rodata) }
    .got : { *(.got) }
    .data : { *(.data) }
    .bss : { *(.bss) }
}
