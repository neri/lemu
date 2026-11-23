ENTRY(_start)
SECTIONS {
 . = 0x100000;
 .text : { *(.text.start) *(.text*) }
 /DISCARD/ : { *(.eh_frame*) *(.comment*) *(.note*) }
}
