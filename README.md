# Lazy Emulator

A Lazy Emulator Package

## Components

### IR86

Intermediate representation for x86 instructions.

This library decodes x86 instructions and converts them into an intermediate representation.

The IR is normalized, so it may not be possible to reconstruct the original instruction exactly from the IR.
Therefore, it is not useful for applications where the exact encoding of the original instruction is important.

### PLUM

User Mode Emulator

## Example Apps

### disasm - Simple Disassembler

- usage
```sh
cargo run -p disasm FILENAME
```

- supported formats:
  - elf32-i386
  - hrb

## MSRV

* The latest version is recommended whenever possible.

## License

MIT License

(c) 2026 Nerry
