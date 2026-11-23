// Shared Node / browser driver. Serve this module next to before.wasm and after.wasm
// for browser use, or pass two snapshot roots to the Node command below.
export const workloads = [
  ['zx8', 128], ['zx16', 128], ['sx8', 128], ['sx16', 128], ['mixed', 128],
  ['alu', 128], ['rmw', 128], ['fib', 1024], ['ret-1', 256], ['ret-4', 64],
  ['ret-depth8', 32], ['ret-16', 16], ['stos', 1024], ['scas', 1024],
];
const median = xs => xs.toSorted((a, b) => a - b)[Math.floor(xs.length / 2)];
export function summarize(rows) {
  return workloads.map(([name]) => {
    const ratios = [0, 1, 2].map(batch => {
      const pair = rows.filter(r => r.batch === batch && r.name === name);
      return 100 * (pair.find(r => r.stage === 1).median_ms / pair.find(r => r.stage === 0).median_ms - 1);
    });
    return { name, change_percent: median(ratios), batches_percent: ratios };
  });
}
export async function compare(bytes, report = () => {}) {
  const hashes = await Promise.all(bytes.map(async b => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', b)), x => x.toString(16).padStart(2, '0')).join('')));
  const modules = await Promise.all(bytes.map(b => WebAssembly.compile(b)));
  const producers = modules.map(m => WebAssembly.Module.customSections(m, 'producers')
    .map(s => new TextDecoder().decode(s)).join(''));
  if (!producers[0] || new Set(producers).size !== 1) throw Error('Compiler mismatch');
  const entries = await Promise.all(modules.map(async m => (await WebAssembly.instantiate(m)).exports));
  const probes = entries.map(e => {
    if (e.load_extend_uop_size() !== 8 || e.load_extend_minor_size() !== 4) throw Error('Layout changed');
    const count = e.run_load_extend_probe();
    if (count !== 1) throw Error(`Probe failed: ${count}`);
    return count;
  });
  report(`Semantic probes: ${probes.join(', ')}; Uop=8, Minor=4`);
  const rows = [];
  for (let batch = 0; batch < 3; batch++) {
    for (let offset = 0; offset < workloads.length; offset++) {
      const caseId = (offset + batch * 5) % workloads.length;
      const [name, iterations] = workloads[caseId];
      const run = stage => entries[stage].bench_load_extend(caseId, iterations);
      const expected = run(0);
      for (let stage = 0; stage < 2; stage++) for (let i = 0; i < 3; i++) {
        if (run(stage) !== expected) throw Error(`Warmup checksum: ${name}`);
      }
      const samples = [[], []];
      for (let sample = 0; sample < 9; sample++) {
        for (const stage of (sample + batch) % 2 ? [1, 0] : [0, 1]) {
          const start = performance.now();
          const actual = run(stage);
          const elapsed = performance.now() - start;
          if (actual !== expected) throw Error(`Checksum: ${name}`);
          samples[stage].push(elapsed);
        }
      }
      for (let stage = 0; stage < 2; stage++) rows.push({
        batch, name, stage, iterations, checksum: expected,
        median_ms: median(samples[stage]), samples_ms: samples[stage],
      });
      report(`batch ${batch + 1}/3 ${name}`);
      // Let the browser paint progress and handle UI between workloads.
      await new Promise(resolve => setTimeout(resolve, 0));
    }
  }
  return { metadata: { producers, probes, wasm_sha256: hashes, date: new Date().toISOString() }, rows, summary: summarize(rows) };
}
if (typeof process !== 'undefined' && process.argv?.[1]?.endsWith('bench-plum-load-extend.mjs')) {
  const { readFileSync, writeFileSync } = await import('node:fs');
  const [output, before, after] = process.argv.slice(2);
  if (!output || !before || !after) throw Error('Usage: node scripts/bench/bench-plum-load-extend.mjs output.json before-root after-root');
  const roots = [before, after];
  const bytes = roots.map(r => readFileSync(`${r}/target/wasm32-unknown-unknown/release/examples/load_extend_bench.wasm`));
  const result = await compare(bytes, console.log);
  Object.assign(result.metadata, { roots, node: process.version,
    affinity: readFileSync('/proc/self/status', 'utf8').split('\n').find(s => s.startsWith('Cpus_allowed_list:')) });
  writeFileSync(output, JSON.stringify(result, null, 2));
  console.log(result.summary);
}
