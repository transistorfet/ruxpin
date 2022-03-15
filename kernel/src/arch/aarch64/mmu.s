
.extern __KERNEL_VIRTUAL_BASE_ADDR

.section .data

.balign 4096

// Translation Table Level 1
_kernel_translation_table_l1:
.quad 0x00000401
.quad 0x40000401
.balign 4096

// Translation Table Level 0
.global _kernel_translation_table_l0
_kernel_translation_table_l0:
.quad _kernel_translation_table_l1 - 0xffff000000000000 + 3
.balign 4096

