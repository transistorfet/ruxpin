
.section .text
.extern kernel_start

.global _start
_start:

	ldr	x4, =0x3F201000
	mov	w5, #0x31
	strb	w5, [x4]

	mrs	x1, MPIDR_EL1
	and	x1, x1, 0x03
	mov	x2, #0
	cmp	x1, x2
	b.ne	L_suspend
	b	L_boot_core

    L_suspend:
	wfe
    L_loop_forever:
	b L_loop_forever

    L_boot_core:
	mov	w5, #0x32
	strb	w5, [x4]

	ldr	x1, =_INIT_STACK_POINTER
	mov	sp, x1
	b kernel_start

_INIT_STACK_POINTER:
	.word	0x90000

