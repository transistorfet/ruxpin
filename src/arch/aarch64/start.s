
.section .text
.extern kernel_start
.extern _default_exceptions_table

/*
 * Kernel Entry Point
 */
.global _start
_start:
	// Print the character '1' on boot, for debugging
	//ldr	x4, =0x3F201000
	//mov	w5, #0x31
	//strb	w5, [x4]

	mrs	x1, MPIDR_EL1
	and	x1, x1, 0x03
	mov	x2, #0
	cmp	x1, x2
	b.ne	L_suspend_core
	b	L_start_kernel

    L_suspend_core:
	// Setup a default stack
	adr	x0, #0x20000
	mul	x0, x0, x1	// The Core ID
	msr	SP_EL0, x0
	msr	SP_EL1, x0
	mov	sp, x0

	bl	_setup_common_system_registers

    L_loop_forever:
	wfe
	b L_loop_forever


    L_start_kernel:
	// Set the stack pointer
	adr	x1, _INIT_STACK_POINTER
	msr	SP_EL0, x1
	msr	SP_EL1, x1
	mov	sp, x1

	bl	_setup_common_system_registers

	// TODO initialize bss


	// Switch Exception Level EL1
	// by setting the flags and return address registers
	mov	x0, #0b00101
	msr	SPSR_EL2, x0

	adr	x2, L_enter_E1
	msr	ELR_EL2, x2
	isb	sy
	eret

    L_enter_E1:
	bl	kernel_start


_setup_common_system_registers:
	mrs	x0, CurrentEL
	lsr	x0, x0, 2
	mov	x1, 3
	cmp	x0, x1
	b.eq	L_setup_EL3
	b	L_setup_EL2

    L_setup_EL3:
	// Set execution state AArch64, EL1 is Non-secure
	mrs	x0, SCR_EL3
	orr	x0, x0, #(1<<10)
	orr	x0, x0, #(1<<0)
	msr	SCR_EL3, x0

	// Configure the exceptions table
	adr	x1, _default_exceptions_table
	msr	VBAR_EL3, x1

	// Disable various traps in EL1
	msr	CPTR_EL3, xzr

	// Switch to EL2
	mov	x0, sp
	msr	SP_EL2, x0
	mov	x0, #0b01001
	msr	SPSR_EL3, x0

	adr	x2, L_setup_EL2
	msr	ELR_EL3, x2
	isb	sy
	eret

    L_setup_EL2:
	// Configure the exceptions table
	adr	x1, _default_exceptions_table
	msr	VBAR_EL1, x1
	msr	VBAR_EL2, x1

	// Configure the Counter
	mov	x0, #(0b11 << 10)
	msr	CNTHCTL_EL2, x0
	msr	CNTVOFF_EL2, xzr

	// Enable Floating Point in EL1/0
	mov	x0, #(0b11 << 20)
	msr	CPACR_EL1, x0

	// Disable various traps in EL1
	mov	x0, #0x3300
	msr	CPTR_EL2, x0
	msr	HSTR_EL2, xzr

	// Enable Aarch64 execution mode
	ldr	x1, =0x80000000
	msr	HCR_EL2, x1
	// Set up various things
	ldr	x1, =0x30C50838
	msr	SCTLR_EL2, x1
	msr	SCTLR_EL1, x1

	isb	sy
	//dsb	sy

	ret


_INIT_STACK_POINTER:
	.word	__KERNEL_END_ADDR + 0x100000

