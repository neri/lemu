# Fixed guest inputs

Binary guests are stored as `.bin.hex` / `.elf.hex` text: two hexadecimal digits
per byte, separated by spaces, with up to 32 bytes per line. Decoding preserves
every original byte. `guest-manifest.json` records the decoded SHA-256 hashes of
the hello ELF and SIMD inputs. Benchmark corpora keep their existing manifests.
Run `make test-tools` to verify all 33 inputs.

Rust examples decode their inputs at compile time with
`plum/examples/support/guest_bytes.rs`. They need no extra build tools. The
Linux32 SSE Wasm runner also decodes its input directly.

For programs that expect a binary file, decode to the ignored build directory:

```sh
python3 scripts/guest-hex.py decode fixtures/hello.elf.hex target/guest-fixtures/hello.elf
```

To capture a deliberately rebuilt input as text:

```sh
python3 scripts/guest-hex.py encode target/guest-fixtures/hello.elf fixtures/hello.elf.hex
```

Keep expected output independent of the emulator under test. Historical
benchmark metadata still names the original binary paths and hashes; those
hashes refer to the decoded bytes, not the text encoding. Source builders emit
hex inputs and keep binary intermediates in temporary directories.

## Frequency corpus (`frequency-v1/`)

`frequency-v1/guest/` holds the frozen freestanding i386 corpus used by the
native and Wasm frequency examples (`frequency_bench`, `flat_frequency_bench`):
14 `.bin.hex` inputs and their sources (`workloads.c`, `start.S`, `link.ld`).
`manifest.json` records the decoded-byte hashes, compiler flags and case
weights; `metadata.json` marks the snapshot as frozen.

To produce another corpus without changing this snapshot:

```sh
python3 scripts/bench/build-plum-frequency-guests.py --output /tmp/plum-frequency-new
```

This needs GCC with i386 support and GNU binutils. Use the frozen hex inputs
for comparisons; a different compiler can produce different bytes.
