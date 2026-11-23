.section .text.start,"ax"
.global _start
_start:
    fninit
    call initialize
repeat:
    call bench
    int $0x40
    jmp repeat
.section .note.GNU-stack,"",@progbits
