.section .text.start,"ax"
.global _start
_start:
    call bench
    int $0x40
    jmp _start
.section .note.GNU-stack,"",@progbits
