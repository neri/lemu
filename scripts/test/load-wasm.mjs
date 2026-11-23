import { readFileSync } from 'node:fs';
const [path, kind] = process.argv.slice(2);
if (!path || !['extend', 'narrow'].includes(kind)) {
    throw Error('Usage: node scripts/test/load-wasm.mjs probe.wasm extend|narrow');
}
const { instance } = await WebAssembly.instantiate(readFileSync(path));
const e = instance.exports;
if (e.load_extend_uop_size() !== 8 || e.load_extend_minor_size() !== 4) {
    throw Error('Uop layout changed');
}
const extended = e.run_load_extend_probe();
if (extended !== 1) throw Error(`Expected 1 load-extension cases, got ${extended}`);
console.log(`${extended} load-extension cases passed in WebAssembly`);
if (kind === 'narrow') {
    const count = e.run_narrow_load_probe();
    if (count !== 1) throw Error(`Expected 1 narrow-load cases, got ${count}`);
    console.log(`${count} narrow-load cases passed in WebAssembly`);
}
