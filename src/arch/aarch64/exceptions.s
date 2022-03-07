

.section .text
.extern fatal_error
.extern handle_exception

.global _default_exceptions_table



.global _create_context
_create_context:
	sub	x0, x0, #(8 * 34)

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

	stp	xzr, xzr, [x0, 240]

	mov	x9, #0x3c1
	stp	x1, x9, [x0, 256]

	adr	x1, PROCESS_SAVED_SP
	str	x0, [x1]
	ret

_save_context:
	sub	x0, x0, #(8 * 34)

	stp	xzr, x1, [x0, 0]
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

	ldp	x0, x1, [sp, 0]
	stp	x0, x1, [x0, 240]

	mrs	x0, ELR_EL1
	mrs	x1, SPSR_EL1
	stp	x0, x1, [x0, 256]

	ret


.global _restore_context
_restore_context:
	ldp	x9, x10, [x0, 256]
	msr	ELR_EL1, x9
	// TODO this is causing an illegal instruction
	//msr	SPSR_EL1, x10

	ldp	x9, x10, [x0, 240]
	stp	x9, x10, [sp, 0]

	ldp	xzr, x1, [x0, 0]
	ldp	x2, x3, [x0, 16]
	ldp	x4, x5, [x0, 32]
	ldp	x6, x7, [x0, 48]
	ldp	x8, x9, [x0, 64]
	ldp	x10, x11, [x0, 80]
	ldp	x12, x13, [x0, 96]
	ldp	x14, x15, [x0, 112]
	ldp	x16, x17, [x0, 128]
	ldp	x18, x19, [x0, 144]
	ldp	x20, x21, [x0, 160]
	ldp	x22, x23, [x0, 176]
	ldp	x24, x25, [x0, 192]
	ldp	x26, x27, [x0, 208]
	ldp	x28, x29, [x0, 224]

	add	x0, x0, #(8 * 34)
	msr	SP_EL0, x0

	ldp	x0, x30, [sp, 0]
	add	sp, sp, #16
	eret

.global _start_multiprocessing
_start_multiprocessing:
	mov	x9, #16
	sub	sp, sp, x9

	ldr	x0, PROCESS_SAVED_SP
	b	_restore_context


_exception_fatal2:
	ldr	x1, =0x3F201000
	mov	w0, #0x24
	strb	w0, [x1]

_exception_fatal:
	ldr	x1, =0x3F201000
	mov	w0, #0x21
	strb	w0, [x1]
	mrs	x0, ESR_EL1
	mrs	x1, ELR_EL1
	b	fatal_error
_loop:
	wfe
	b	_loop

.global PROCESS_SAVED_SP
PROCESS_SAVED_SP:
	.word	0

.macro HANDLE_CONTEXT_SWITCH
	sub	sp, sp, #16
	stp	x0, x30, [sp, 0]

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

	mrs	x0, SP_EL0
	bl	_save_context
	adr	x1, PROCESS_SAVED_SP
	str	x0, [x1]

	mrs	x1, ESR_EL1
	mrs	x2, FAR_EL1
	bl	handle_exception

	ldr	x0, PROCESS_SAVED_SP
	b	_restore_context

.endm


/*
 * Exceptions Table
 */
.balign 2048
_default_exceptions_table:

// Exceptions where SP_EL0 is the stack
.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

// Exceptions where SP_ELx is the stack
.balign 0x80
	HANDLE_CONTEXT_SWITCH

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

// Exceptions from lower EL in AArch64
.balign 0x80
	HANDLE_CONTEXT_SWITCH

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

// Exceptions from lower EL in AArch32
.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal

.balign 0x80
	b	_exception_fatal


