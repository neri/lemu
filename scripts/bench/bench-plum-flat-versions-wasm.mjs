// Compare flat-mode revisions in one Node process with identical checksums.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const [before, after, output] = process.argv.slice(2);
if (!before || !after || !output) throw Error('Usage: node scripts/bench/bench-plum-flat-versions-wasm.mjs before.wasm after.wasm output.json');
const paths = [before, after];
const modules = await Promise.all(paths.map(p => WebAssembly.compile(readFileSync(p))));
const producers = modules.map(m => WebAssembly.Module.customSections(m, 'producers').map(s => new TextDecoder().decode(s)).join(''));
if (!producers[0] || producers[0] !== producers[1]) throw Error('Compiler mismatch');
const entries = await Promise.all(modules.map(async m => (await WebAssembly.instantiate(m)).exports));
const rows = [];
for (let round = 0; round < 3; round++) {
  for (let c = 0; c < 4; c++) {
    const iterations = c < 2 ? 64 : 16384;
    const samples = [[], []];
    let expected;
    for (let sample = 0; sample < 9; sample++) {
      for (const stage of (sample + round) % 2 ? [1, 0] : [0, 1]) {
        const e = entries[stage];
        // Archived pre-rename artifacts keep their original export names.
        const prepare = e.prepare_flat_bench ?? e.prepare_unified_bench;
        const run = e.run_flat_bench ?? e.run_unified_bench;
        prepare(c, 1);
        run(c, 2);
        const start = performance.now();
        const checksum = run(c, iterations);
        samples[stage].push(performance.now() - start);
        expected ??= checksum;
        if (checksum !== expected) throw Error('Checksum mismatch');
      }
    }
    const medians = samples.map(s => s.toSorted((a, b) => a - b)[4]);
    rows.push({ round, case: c, iterations, checksum: expected, samples_ms: samples, medians_ms: medians });
    console.log(JSON.stringify({ round, case: c, medians_ms: medians, ratio: medians[1] / medians[0] }));
  }
}
writeFileSync(output, JSON.stringify({ node: process.version, paths, producers, hashes: paths.map(p => createHash('sha256').update(readFileSync(p)).digest('hex')), rows }, null, 2));
