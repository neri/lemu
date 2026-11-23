// Pass workspace snapshot roots containing target/wasm32-unknown-unknown/release/examples.
// Samples are alternated in one Node process; JSON retains all measurements.
import { readFileSync, writeFileSync } from 'node:fs';
const guardsOnly = process.argv.includes("--guards");
const selectedCases = process.argv.find(x => x.startsWith("--cases="))?.slice(8).split(",").map(Number);
const [output, ...roots] = process.argv.slice(2).filter(x => !x.startsWith("--"));
if (!output || roots.length < 2) throw Error('Usage: node scripts/bench/bench-plum-rmw-wasm.mjs output.json before-root after-root [other-root]');
const entries = [];
const producers = [];
for (const root of roots) {
  const exports = {};
  for (const name of ['rmw_dispatch_bench', 'cpu_dispatch_bench', 'return_dispatch_bench', 'string_bench']) {
    const bytes = readFileSync(`${root}/target/wasm32-unknown-unknown/release/examples/${name}.wasm`);
    const { module, instance } = await WebAssembly.instantiate(bytes);
    if (name === 'rmw_dispatch_bench') {
      producers.push(WebAssembly.Module.customSections(module, 'producers')
        .map(section => new TextDecoder().decode(section)).join(''));
    }
    Object.assign(exports, instance.exports);
  }
  if (exports.uop_size() !== 8 || exports.minor_size() !== 4) throw Error('Uop layout changed');
  entries.push(exports);
}
if (new Set(producers).size !== 1 || !producers[0]) {
  throw Error('Use matching compiler producers for every comparison stage');
}
const cases = [];
for (let kind = 0; !guardsOnly && kind < 119; kind++) for (let scenario = 0; scenario < 6; scenario++) {
  if (selectedCases && !selectedCases.includes(kind)) continue;
  cases.push({ name: `rmw-${kind}-${scenario}`, run: e => e.bench_rmw(kind, scenario, 8) });
}
cases.push(
  { name: 'fib', run: e => e.bench_cpu_dispatch(guardsOnly ? 1024 : 64) },
  { name: 'return', run: e => e.bench_return_dispatch(0, guardsOnly ? 256 : 16) },
  { name: 'stos', run: e => e.bench_string(1, 4, 65536, 0, 0, 0, guardsOnly ? 1024 : 128) },
  { name: 'scas', run: e => e.bench_string(4, 1, 65536, 0, 0, 0, guardsOnly ? 1024 : 128) },
);
const rows = [];
const metadata = { roots, producers, selectedCases, guardsOnly, node: process.version, date: new Date().toISOString(), affinity: readFileSync('/proc/self/status', 'utf8').split('\n').find(l => l.startsWith('Cpus_allowed_list:')) };
console.log(JSON.stringify(metadata));
for (let batch = 0; batch < 3; batch++) {
  for (let index = 0; index < cases.length; index++) {
    // Rotate groups between batches to avoid making the same workload always cold.
    const c = cases[(index + batch * 239) % cases.length];
    const expected = c.run(entries[0]);
    const samples = entries.map(() => []);
    for (const e of entries) for (let i = 0; i < 3; i++) if (c.run(e) !== expected) throw Error(`Checksum: ${c.name}`);
    for (let sample = 0; sample < 9; sample++) {
      const order = entries.map((_, i) => i);
      if (sample % 2) order.reverse();
      for (const stage of order) {
        const start = performance.now();
        if (c.run(entries[stage]) !== expected) throw Error(`Checksum: ${c.name}`);
        samples[stage].push(performance.now() - start);
      }
    }
    for (let stage = 0; stage < entries.length; stage++) {
      rows.push({ batch, name: c.name, stage, checksum: expected, median_ms: samples[stage].toSorted((a, b) => a - b)[4], samples_ms: samples[stage] });
    }
    if (index % 100 === 0) console.log(`batch ${batch + 1}/3: ${index}/${cases.length}`);
  }
  writeFileSync(output, JSON.stringify({ metadata, rows }, null, 2));
}
console.log(`Saved ${rows.length} rows to ${output}`);
