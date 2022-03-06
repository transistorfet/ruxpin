

.section .text
.extern fatal_error

.global _default_exceptions_table

.macro EXCEPTION_FATAL
	ldr	x1, =0x3F201000
	mov	w0, #0x21
	strb	w0, [x1]
	mrs	x0, ESR_EL1
	mrs	x1, ELR_EL1
	b	fatal_error

	b	_loop
.endm

_loop:
	b	_loop


/*
 * Exceptions Table
 */
.balign 2048
_default_exceptions_table:

// Exceptions for EL0
.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

// Exceptions for ELx
.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

// Exceptions for ...
.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

// Exceptions for ...
.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL

.balign 0x80
	EXCEPTION_FATAL


