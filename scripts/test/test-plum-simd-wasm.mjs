// Build: cargo build -p plum --release --target wasm32-unknown-unknown \
//   --no-default-features --features sse --example simd_probe
// Compares the SSE guests' checksum in WebAssembly with the native one.
import { readFileSync } from 'node:fs';
const [path, expected] = process.argv.slice(2);
if (!path || expected === undefined) {
  throw Error('Usage: node scripts/test/test-plum-simd-wasm.mjs simd_probe.wasm native-checksum');
}
const { instance } = await WebAssembly.instantiate(readFileSync(path));
const checksum = instance.exports.simd_checksum() >>> 0;
if (checksum !== 4) throw Error(`Expected 4 fixed numeric checks, got ${checksum}`);
if (checksum !== Number(expected)) throw Error(`Wasm checksum ${checksum}, native ${expected}`);
console.log(`Fixed SIMD contracts match native in WebAssembly (checksum ${checksum})`);
