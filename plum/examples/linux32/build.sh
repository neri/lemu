#!/bin/sh
# Run from the workspace root; needs gcc + binutils, no 32-bit libc.
set -eu
mkdir -p target/linux32
"${CC:-gcc}" -m32 -march=i386 -O2 -ffreestanding -fno-pie -fno-pic \
  -fno-stack-protector -fno-builtin -nostdlib -static \
  -Wl,--build-id=none -Wl,-z,noexecstack \
  plum/examples/linux32/jit_calc.c -o target/linux32/jit-calc
