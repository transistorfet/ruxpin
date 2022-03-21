__APPLICATION_START_ADDR = 0x400000;
PAGESIZE = 4096;
 
ENTRY(_start)
/* SEARCH_DIR("=/usr/local/lib/aarch64-linux-gnu"); */
SECTIONS
{
    . = __APPLICATION_START_ADDR;

    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .got : { *(.got*) }
    .rela : { *(.rela*) }
    .data : { *(.data*) }
    .bss : { *(.bss*) }

    __APPLICATION_END_ADDR = .;
}

