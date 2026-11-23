// Build: cargo build -p plum --release --target wasm32-unknown-unknown \
//   --no-default-features --features sse --example linux32
// Runs the gcc-compiled SSE2 program (plum/examples/support/simd/sse-calc.elf.hex)
// and compares its output with an x86 CPU's (sse-calc.out).
import { readFileSync } from 'node:fs';
const [wasm, elfPath, expectedPath] = process.argv.slice(2);
const hex = readFileSync(elfPath, 'utf8').replace(/[ \t\r\n]/g, '');
if (!/^(?:[0-9a-fA-F]{2})*$/.test(hex)) throw Error(`Invalid guest hex: ${elfPath}`);
const elf = Buffer.from(hex, 'hex');
const expected = readFileSync(expectedPath, 'utf8');
const { instance } = await WebAssembly.instantiate(readFileSync(wasm));
const e = instance.exports;
const ptr = e.linux32_prepare(elf.length, 0) >>> 0;
new Uint8Array(e.memory.buffer, ptr, elf.length).set(elf);
const code = e.linux32_run();
const decode = (p, n) => new TextDecoder().decode(new Uint8Array(e.memory.buffer, p, n));
const out = decode(e.linux32_stdout_ptr() >>> 0, e.linux32_stdout_len());
const err = decode(e.linux32_stderr_ptr() >>> 0, e.linux32_stderr_len());
if (code !== 0 || err || out !== expected) throw Error(JSON.stringify({ code, err, out }));
console.log(`gcc-compiled SSE2 program matches an x86 CPU in WebAssembly (${out.split('\n').length - 1} lines)`);
