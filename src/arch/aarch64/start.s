
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
	// Set the stack pointer
	adr	x1, _INIT_STACK_POINTER
	msr	SP_EL0, x1
	msr	SP_EL1, x1
	msr	SP_EL2, x1
	mov	sp, x1

	// Configure the exceptions table
	adr	x1, _default_exceptions_table
	msr	VBAR_EL1, x1
	msr	VBAR_EL2, x1
	msr	VBAR_EL3, x1

	// TODO initialize bss

	// Enable Floating Point in EL1/0
	mov	x0, #(0b11 << 20)
	msr	CPACR_EL1, x0

	// Disable various traps in EL1
	mov	x0, #0x3300
	msr	CPTR_EL2, x0
	msr	HSTR_EL2, xzr

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

	// Switch Exception Level EL1
	// by setting the flags and return address registers
	mov	x0, #0b00101
	msr	SPSR_EL3, x0

	adr	x2, L_enter_E2
	msr	ELR_EL3, x2
	eret

    L_enter_E2:
	bl	kernel_start


.global _get_current_el
_get_current_el:
	mrs	x0, CurrentEL
	lsr	x0, x0, 2
	ret


// TODO replace this with something configurable
_INIT_STACK_POINTER:
	.word	0x90000

