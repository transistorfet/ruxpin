
.section .text
.extern kernel_start
.extern _default_exceptions_table

/*
 * Kernel Entry Point
 */
.global _start
_start:

	//ldr	x4, =0x3F201000
	//mov	w5, #0x31
	//strb	w5, [x4]

	mrs	x1, MPIDR_EL1
	and	x1, x1, 0x03
	mov	x2, #0
	cmp	x1, x2
	b.ne	L_suspend_core
	b	L_start_core

    L_suspend_core:
	wfe
    L_loop_forever:
	b L_loop_forever

    L_start_core:
	// Configure the exceptions table
	adr	x1, _default_exceptions_table
	msr	VBAR_EL1, x1
	msr	VBAR_EL2, x1
	msr	VBAR_EL3, x1

	// Set the stack pointer
	adr	x1, _INIT_STACK_POINTER
	msr	SP_EL0, x1
	msr	SP_EL1, x1
	msr	SP_EL2, x1
	mov	sp, x1

	bl	_print_current_el

	// Set up necessary modes
	ldr	x1, =0x80000000
	msr	HCR_EL2, x1
	ldr	x1, =0x30C50838
	msr	SCTLR_EL2, x1
	msr	SCTLR_EL1, x1

	// Set execution state AArch64, EL1 is Non-secure
	mrs	x0, SCR_EL3
	orr	x0, x0, #(1<<10)
	orr	x0, x0, #(1<<0)
	msr	SCR_EL3, x0

	// Switch Exception Level (EL)
	// by setting the flags and return address registers
	//mov	x1, #0x03C5	// EL1
	//mov	x1, #0x03C9	// EL2
	//mov	x1, #0x03CD	// EL3
	//msr	SPSR_EL3, x1
	mov	x0, #0b00101
	msr	SPSR_EL3, x0

	adr	x2, L_enter_E2
	msr	ELR_EL3, x2
	eret

    L_enter_E2:
	bl	_print_current_el

	bl	kernel_start


_print_current_el:
	ldr	x4, =0x3F201000
	mov	w5, #0x30
	mrs	x6, CurrentEL
	lsr	x6, x6, 2
	add	w5, w5, w6
	strb	w5, [x4]
	ret

.global _get_current_el
_get_current_el:
	mrs	x0, CurrentEL
	ret


// TODO replace this with something configurable
_INIT_STACK_POINTER:
	.word	0x90000

