# SSE fixtures

Fixed machine-code inputs are stored as plain hexadecimal text. Rust examples
use `support/guest_bytes.rs` to decode them at compile time; the Wasm runner
reads the ELF hex directly. No assembler or cross-compiler is needed to use them.
`make test-tools` checks the decoded bytes against their original SHA-256 hashes.

- `bench.s` / `bench.bin.hex`: instructions used only for timing workloads.
- `sse-calc.elf.hex`: `plum/examples/linux32/sse_calc.c` compiled for SSE2.
- `sse-calc.out`: expected output recorded on an Intel Core i9-12900H.
  Keep this independent reference; do not regenerate it using PLUM.

To rebuild the timing input from the workspace root (GNU binutils required):

```sh
mkdir -p target/guest-fixtures
as --32 -o target/guest-fixtures/bench.o plum/examples/support/simd/bench.s
objcopy -O binary -j .text target/guest-fixtures/bench.o target/guest-fixtures/bench.bin
python3 scripts/guest-hex.py encode target/guest-fixtures/bench.bin plum/examples/support/simd/bench.bin.hex
```

To rebuild the ELF (GCC with i386 support required):

```sh
mkdir -p target/guest-fixtures
gcc -m32 -march=pentium4 -msse2 -mfpmath=sse -O3 -fno-math-errno \
  -ffreestanding -fno-tree-loop-distribute-patterns -fno-pie -fno-pic \
  -fno-stack-protector -nostdlib -static -Wl,--build-id=none \
  -Wl,-z,noexecstack plum/examples/linux32/sse_calc.c \
  -o target/guest-fixtures/sse-calc.elf
python3 scripts/guest-hex.py encode target/guest-fixtures/sse-calc.elf plum/examples/support/simd/sse-calc.elf.hex
```

A different compiler may produce different bytes. Review the code and update
`fixtures/guest-manifest.json` deliberately when changing an input. On x86 Linux
with i386 execution enabled, the generated ELF can be run directly to compare
its output against the saved `sse-calc.out`.
