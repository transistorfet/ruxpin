

.section .text
.extern fatal_error

.global _default_exceptions_table

.macro EXCEPTION_FATAL
	//ldr	x0, =0x80000
	//mov	sp, x0
	//bl	fatal_error
	//eret

	ldr	x1, =0x3F201000
	mov	w0, #0x21
	strb	w0, [x1]

	b	_loop
.endm

_loop:
	b	_loop

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


.global _trigger_illegal_instruction
_trigger_illegal_instruction:
	.word	0x000000

