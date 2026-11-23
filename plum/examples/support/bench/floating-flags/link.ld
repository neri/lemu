ENTRY(_start)
SECTIONS {
 . = 0x100000;
 .text : { *(.text.start) *(.text*) }
 .rodata : { *(.rodata*) }
 .data : { *(.data*) }
 . = 0x20000;
 .output (NOLOAD) : { *(.output) }
 . = 0x30000;
 .bss (NOLOAD) : { *(.bss*) *(COMMON) }
 /DISCARD/ : { *(.eh_frame*) *(.comment*) *(.note*) }
}
