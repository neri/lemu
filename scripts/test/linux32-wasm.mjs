import { readFileSync } from 'node:fs';
const [wasm = 'target/wasm32-unknown-unknown/release/examples/linux32.wasm', elfPath = 'target/linux32/jit-calc'] = process.argv.slice(2);
const elf = readFileSync(elfPath);
const { instance } = await WebAssembly.instantiate(readFileSync(wasm));
const e = instance.exports;
const decode = (ptr, len) => new TextDecoder().decode(new Uint8Array(e.memory.buffer, ptr, len));
function run(input, file = elf) {
    const ptr = e.linux32_prepare(file.length, input.length) >>> 0;
    new Uint8Array(e.memory.buffer, ptr, file.length).set(file);
    new Uint8Array(e.memory.buffer, ptr + file.length, input.length).set(input);
    const code = e.linux32_run();
    return { code, out: decode(e.linux32_stdout_ptr() >>> 0, e.linux32_stdout_len()), err: decode(e.linux32_stderr_ptr() >>> 0, e.linux32_stderr_len()) };
}
const encode = s => new TextEncoder().encode(s);
const cases = [[6, 7, 5], [0, 0, 0], [17, 23, 101], [999999, 999999, 999999]];
for (const [a, b, x] of cases) {
    const actual = run(encode(`${a} ${b} ${x}\n`));
    const expected = `initial: ${(Math.imul(a, x) + b) >>> 0}\npatched: ${(Math.imul(a + 1, x) + b + 2) >>> 0}\nreplaced: 99\n`;
    if (actual.code !== 0 || actual.out !== expected || actual.err) throw Error(JSON.stringify({ actual, expected }));
    console.log(`${a} ${b} ${x}: ${actual.out.trim().replaceAll('\n', '; ')}`);
}
const input = new Uint8Array([...encode('6 7 5\n'), 0xb8, 123, 0, 0, 0, 0xc3]);
const patched = run(input);
if (patched.code !== 0 || patched.out !== 'initial: 37\npatched: 44\nreplaced: 99\nhost patched: 123\n' || patched.err) throw Error(JSON.stringify(patched));
const invalid = run(new Uint8Array(), elf.subarray(0, 40));
if (invalid.code !== -1 || !invalid.err || invalid.out) throw Error('Malformed ELF was accepted');
console.log('6 Linux/i386 ELF scenarios passed in Wasm (including host read into cached code)');
