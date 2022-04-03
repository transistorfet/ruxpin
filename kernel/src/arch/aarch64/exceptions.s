

.extern fatal_error
.extern handle_irq
.extern handle_exception


.section .text

.global _create_context
_create_context:
	// Integer Registers
	stp	xzr, xzr, [x0, 0]
	stp	xzr, xzr, [x0, 16]
	stp	xzr, xzr, [x0, 32]
	stp	xzr, xzr, [x0, 48]
	stp	xzr, xzr, [x0, 64]
	stp	xzr, xzr, [x0, 80]
	stp	xzr, xzr, [x0, 96]
	stp	xzr, xzr, [x0, 112]
	stp	xzr, xzr, [x0, 128]
	stp	xzr, xzr, [x0, 144]
	stp	xzr, xzr, [x0, 160]
	stp	xzr, xzr, [x0, 176]
	stp	xzr, xzr, [x0, 192]
	stp	xzr, xzr, [x0, 208]
	stp	xzr, xzr, [x0, 224]
	stp	xzr, x1, [x0, 240]	// Initial value of SP

	add	x0, x0, #256

	// Floating Point Registers
	stp	xzr, xzr, [x0, 0]
	stp	xzr, xzr, [x0, 32]
	stp	xzr, xzr, [x0, 64]
	stp	xzr, xzr, [x0, 96]
	stp	xzr, xzr, [x0, 128]
	stp	xzr, xzr, [x0, 160]
	stp	xzr, xzr, [x0, 192]
	stp	xzr, xzr, [x0, 224]
	stp	xzr, xzr, [x0, 256]
	stp	xzr, xzr, [x0, 288]
	stp	xzr, xzr, [x0, 320]
	stp	xzr, xzr, [x0, 352]
	stp	xzr, xzr, [x0, 384]
	stp	xzr, xzr, [x0, 416]
	stp	xzr, xzr, [x0, 448]
	stp	xzr, xzr, [x0, 480]

	add	x0, x0, #512

	// Additional Control Registers
	mov	x9, #0x0		// Default value for PSTATE
	stp	x2, x9, [x0, 0]	// Push the initial PC and PSTATE values

	sub	x0, x0, #(512 + 256)

	ret


.global _start_multitasking
_start_multitasking:
	ldr	x0, CURRENT_CONTEXT
	b	_restore_context


_save_context:
	// Integer Registers
	stp	x2, x3, [x0, 16]
	stp	x4, x5, [x0, 32]
	stp	x6, x7, [x0, 48]
	stp	x8, x9, [x0, 64]
	stp	x10, x11, [x0, 80]
	stp	x12, x13, [x0, 96]
	stp	x14, x15, [x0, 112]
	stp	x16, x17, [x0, 128]
	stp	x18, x19, [x0, 144]
	stp	x20, x21, [x0, 160]
	stp	x22, x23, [x0, 176]
	stp	x24, x25, [x0, 192]
	stp	x26, x27, [x0, 208]
	stp	x28, x29, [x0, 224]

	// Save the two values on the stack now that we have temp regs
	ldp	x8, x9, [sp, 0]
	stp	x8, x1, [x0, 0]
	mrs	x8, SP_EL0
	stp	x9, x8, [x0, 240]

	// Reposition offset
	add	x0, x0, #256

	// Floating Point Registers
	stp	q0, q1, [x0, 0]
	stp	q2, q3, [x0, 32]
	stp	q4, q5, [x0, 64]
	stp	q6, q7, [x0, 96]
	stp	q8, q9, [x0, 128]
	stp	q10, q11, [x0, 160]
	stp	q12, q13, [x0, 192]
	stp	q14, q15, [x0, 224]
	stp	q16, q17, [x0, 256]
	stp	q18, q19, [x0, 288]
	stp	q20, q21, [x0, 320]
	stp	q22, q23, [x0, 352]
	stp	q24, q25, [x0, 384]
	stp	q26, q27, [x0, 416]
	stp	q28, q29, [x0, 448]
	stp	q30, q31, [x0, 480]

	add	x0, x0, #512

	mrs	x8, ELR_EL1
	mrs	x9, SPSR_EL1
	stp	x8, x9, [x0, 0]

	sub	x0, x0, #(512 + 256)

	ret


_restore_context:
	// Indexing can only have an offset 512, so advance the pointer to reach the rest
	add	x0, x0, #(512 + 256)

	ldr	x9, [x0, 16]
	msr	TTBR0_EL1, x9

	ldp	x9, x10, [x0, 0]
	msr	ELR_EL1, x9
	msr	SPSR_EL1, x10

	sub	x0, x0, #512

	// Floating Point Registers
	ldp	q30, q31, [x0, 480]
	ldp	q28, q29, [x0, 448]
	ldp	q26, q27, [x0, 416]
	ldp	q24, q25, [x0, 384]
	ldp	q22, q23, [x0, 352]
	ldp	q20, q21, [x0, 320]
	ldp	q18, q19, [x0, 288]
	ldp	q16, q17, [x0, 256]
	ldp	q14, q15, [x0, 224]
	ldp	q12, q13, [x0, 192]
	ldp	q10, q11, [x0, 160]
	ldp	q8, q9, [x0, 128]
	ldp	q6, q7, [x0, 96]
	ldp	q4, q5, [x0, 64]
	ldp	q2, q3, [x0, 32]
	ldp	q0, q1, [x0, 0]

	sub	x0, x0, #256

	// Integer Registers
	ldp	x30, x9, [x0, 240]
	msr	SP_EL0, x9

	ldp	x28, x29, [x0, 224]
	ldp	x26, x27, [x0, 208]
	ldp	x24, x25, [x0, 192]
	ldp	x22, x23, [x0, 176]
	ldp	x20, x21, [x0, 160]
	ldp	x18, x19, [x0, 144]
	ldp	x16, x17, [x0, 128]
	ldp	x14, x15, [x0, 112]
	ldp	x12, x13, [x0, 96]
	ldp	x10, x11, [x0, 80]
	ldp	x8, x9, [x0, 64]
	ldp	x6, x7, [x0, 48]
	ldp	x4, x5, [x0, 32]
	ldp	x2, x3, [x0, 16]
	ldp	x0, x1, [x0, 0]

	// Invalidate the TLB cache in case TTBR0_EL1 points to a different table
	tlbi	VMALLE1IS
	dsb	ish
	isb

	eret


_exception_fatal:
	// Restore the kernel translation table so we can directly access lower memory
	mrs	x0, TTBR1_EL1
	msr	TTBR0_EL1, x0

	// Print a ! character (for debugging when printing from rust causes exceptions)
	ldr	x1, =0xFFFF00003F201000
	mov	w0, #0x21
	strb	w0, [x1]

	// Jump to the fatal error code
	mrs	x0, ELR_EL1
	mrs	x1, ESR_EL1
	mrs	x2, FAR_EL1
	b	fatal_error
_loop:
	wfe
	b	_loop


// Handle an exception from EL0 to EL1 (save the user process context)
.macro HANDLE_CONTEXT_SWITCH handler
	// Save two register values before using the registers for temporary values
	sub	sp, sp, #16
	stp	x0, x30, [sp, 0]

	// Restore the kernel translation table so we can directly access lower memory
	// TODO this should be avoided if possible I think, for performance reasons.  I'm not actually invalidating the TLB
	//	here either, so there could be an invalid reference if not careful.  It'd be best if we can modify all the
	//	addresses in the kernel so that we don't need to do this
	//mrs	x0, TTBR1_EL1
	//msr	TTBR0_EL1, x0

	// EL2/EL3 will cause a fatal error for now
	mrs	x0, CurrentEL
	mov	x30, #4
	cmp	x0, x30
	b.ne	_exception_fatal

	// Save the user process's context (and subtract the values stored at the start from the stack)
	ldr	x0, CURRENT_CONTEXT
	bl	_save_context
	add	sp, sp, #16

	// Call the handler with exception-identifying information
	mrs	x1, ELR_EL1
	mrs	x2, ESR_EL1
	mrs	x3, FAR_EL1
	mrs	x4, SP_EL0
	bl	\handler

	// Restore the context and return the user process
	ldr	x0, CURRENT_CONTEXT
	b	_restore_context
.endm

// Handle an exception from EL1 to EL1 (ie. the kernel is already running,
// save kernel registers on the stack instead of the process context).
.macro HANDLE_KERNEL_EXCEPTION handler
	// Print a $ character (for debugging when printing from rust causes exceptions)
	ldr	x1, =0xFFFF00003F201000
	mov	w0, #0x24
	strb	w0, [x1]

	add	sp, sp, #160
	stp	x0, x1, [sp, 0]
	stp	x2, x3, [sp, 16]
	stp	x4, x5, [sp, 32]
	stp	x6, x7, [sp, 48]
	stp	x8, x9, [sp, 64]
	stp	x10, x11, [sp, 80]
	stp	x12, x13, [sp, 96]
	stp	x14, x15, [sp, 112]
	stp	x16, x17, [sp, 128]
	stp	x18, x30, [sp, 144]

	mrs	x1, ESR_EL1
	mrs	x2, ELR_EL1
	mrs	x3, FAR_EL1
	bl	\handler

	stp	x18, x30, [sp, 144]
	stp	x16, x17, [sp, 128]
	stp	x14, x15, [sp, 112]
	stp	x12, x13, [sp, 96]
	stp	x10, x11, [sp, 80]
	stp	x8, x9, [sp, 64]
	stp	x6, x7, [sp, 48]
	stp	x4, x5, [sp, 32]
	stp	x2, x3, [sp, 16]
	stp	x0, x1, [sp, 0]
	sub	sp, sp, #160

	eret
.endm


/*
 * Exceptions Table
 */
.balign 4096
.global _default_exceptions_table
_default_exceptions_table:

// Exceptions where SP_EL0 is the stack
.balign 0x80	// Synchronous
	b	_exception_fatal

.balign 0x80	// IRQ
	b	_exception_fatal

.balign 0x80	// Fast IRQ
	b	_exception_fatal

.balign 0x80	// SError
	b	_exception_fatal

// Exceptions where SP_ELx is the stack
.balign 0x80	// Synchronous
	b	_exception_fatal

.balign 0x80	// IRQ
	HANDLE_KERNEL_EXCEPTION handle_irq

.balign 0x80	// Fast IRQ
	b	_exception_fatal

.balign 0x80	// SError
	b	_exception_fatal

// Exceptions from lower EL in AArch64
.balign 0x80	// Synchronous
	HANDLE_CONTEXT_SWITCH handle_exception

.balign 0x80	// IRQ
	HANDLE_CONTEXT_SWITCH handle_irq

.balign 0x80	// Fast IRQ
	b	_exception_fatal

.balign 0x80	// SError
	b	_exception_fatal

// Exceptions from lower EL in AArch32
.balign 0x80	// Synchronous
	b	_exception_fatal

.balign 0x80	// IRQ
	b	_exception_fatal

.balign 0x80	// Fast IRQ
	b	_exception_fatal

.balign 0x80	// SError
	b	_exception_fatal


