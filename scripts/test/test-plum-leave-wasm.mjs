import { readFileSync } from 'node:fs';
const path = process.argv[2];
if (!path) throw Error('Usage: node scripts/test/test-plum-leave-wasm.mjs leave_probe.wasm');
const { instance } = await WebAssembly.instantiate(readFileSync(path));
const count = instance.exports.leave_checks();
if (count !== 2) throw Error(`Expected 2 LEAVE fault/retry contracts, got ${count}`);
console.log('Harvard and Flat LEAVE fault/retry contracts passed in WebAssembly');
