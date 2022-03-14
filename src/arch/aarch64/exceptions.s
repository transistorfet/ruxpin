

.extern fatal_error
.extern handle_exception
.extern PROCESS_SAVED_SP


.section .text

.global create_context
create_context:
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

	mov	x9, #0x0		// Default value for PSTATE
	stp	x2, x9, [x0, 256]	// Push the initial PC and PSTATE values

	ret


.global start_multitasking
start_multitasking:
	ldr	x0, CURRENT_CONTEXT
	b	_restore_context


_save_context:
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

	ldp	x8, x9, [sp, 0]
	stp	x8, x1, [x0, 0]
	mrs	x8, SP_EL0
	stp	x9, x8, [x0, 240]

	mrs	x8, ELR_EL1
	mrs	x9, SPSR_EL1
	stp	x8, x9, [x0, 256]

	ret


_restore_context:
	ldr	x9, [x0, 272]
	msr	TTBR0_EL1, x9

	ldp	x9, x10, [x0, 256]
	msr	ELR_EL1, x9
	msr	SPSR_EL1, x10

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

	// TODO invalidate the TLB cache
	isb
	tlbi	VMALLE1IS

	eret


_exception_fatal:
	ldr	x1, =0xFFFF00003F201000
	mov	w0, #0x21
	strb	w0, [x1]
	mrs	x0, ESR_EL1
	mrs	x1, ELR_EL1
	b	fatal_error
_loop:
	wfe
	b	_loop


.macro HANDLE_CONTEXT_SWITCH
	// Save two register values before using the registers for temporary values
	sub	sp, sp, #16
	stp	x0, x30, [sp, 0]

	// Restore the kernel translation table so we can directly access lower memory
	mrs	x0, TTBR1_EL1
	msr	TTBR0_EL1, x0

	// EL2/EL3 will cause a fatal error for now
	mrs	x0, CurrentEL
	mov	x30, #4
	cmp	x0, x30
	b.ne	_exception_fatal

	// If we didn't come from EL0, then cause a fatal error for now
	mrs	x0, SPSR_EL1
	and	x0, x0, #0x0F
	lsr	x0, x0, 2
	cmp	x0, xzr
	b.ne	_exception_fatal

	ldr	x0, CURRENT_CONTEXT
	bl	_save_context
	add	sp, sp, #16

	mrs	x1, ESR_EL1
	mrs	x2, ELR_EL1
	mrs	x3, FAR_EL1
	bl	handle_exception

	ldr	x0, CURRENT_CONTEXT
	b	_restore_context

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
	b	_exception_fatal

.balign 0x80	// Fast IRQ
	b	_exception_fatal

.balign 0x80	// SError
	b	_exception_fatal

// Exceptions from lower EL in AArch64
.balign 0x80	// Synchronous
	HANDLE_CONTEXT_SWITCH

.balign 0x80	// IRQ
	b	_exception_fatal

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


