import { readFileSync } from 'node:fs';
const path = process.argv[2];
if (!path) throw Error('Usage: node scripts/test/test-plum-rmw-wasm.mjs path/to/rmw_dispatch_bench.wasm');
const { instance } = await WebAssembly.instantiate(readFileSync(path));
const e = instance.exports;
if (e.uop_size() !== 8 || e.minor_size() !== 4) throw Error('Uop layout changed');
const count = e.run_rmw_probe();
if (count !== 1) throw Error(`Expected 1 checks, got ${count}`);
console.log(`RMW: ${count} checks; Uop=${e.uop_size()} bytes, Minor=${e.minor_size()} bytes`);
