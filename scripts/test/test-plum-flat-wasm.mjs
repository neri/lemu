import { readFileSync } from 'node:fs';
const path = process.argv[2];
if (!path) throw Error('Usage: node scripts/test/test-plum-flat-wasm.mjs probe.wasm');
const { instance } = await WebAssembly.instantiate(readFileSync(path));
const count = instance.exports.flat_memory_checks();
if (count !== 1) throw Error(`Expected 1 code-update contract, got ${count}`);
console.log('Flat tracked-code-update contract passed in WebAssembly');
