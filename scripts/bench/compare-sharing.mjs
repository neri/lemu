// Compare the saved pre-extraction tree and final candidate with identical guests.
// node ... before/artifacts after/artifacts output.json [fpu|guards|frequency|stress]
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
const [before, after, output, suite = 'fpu'] = process.argv.slice(2);
if (!output) throw Error('before-dir after-dir output.json [fpu|guards|frequency|stress]');
const dirs = [before, after];
const allowedCpu = readFileSync('/proc/self/status', 'utf8').match(/^Cpus_allowed_list:\s*(.*)$/m)[1].split(/[-,]/)[0];
execFileSync('taskset', ['-pc', allowedCpu, String(process.pid)]);
const hashes = {};
const rows = [];
const metadata = {
  node: process.version, rustc: execFileSync('rustc', ['-Vv'], { encoding: 'utf8' }),
  affinity: readFileSync('/proc/self/status', 'utf8').match(/^Cpus_allowed_list:\s*(.*)$/m)?.[1],
  dirs, rounds: 3, samples: 7, warmups: 3, timing: 'warm cache; initialization excluded',
};
function hash(path) { hashes[path] = createHash('sha256').update(readFileSync(path)).digest('hex'); }
function save() { writeFileSync(output, JSON.stringify({ metadata, hashes, rows }, null, 2) + '\n'); }
async function exportsFor(name) {
  return Promise.all(dirs.map(async dir => {
    const path = join(dir, name + '.wasm'); hash(path);
    const module = await WebAssembly.compile(readFileSync(path));
    const producers = WebAssembly.Module.customSections(module, 'producers').map(s => new TextDecoder().decode(s)).join('');
    if (!metadata.producers) metadata.producers = producers;
    if (metadata.producers !== producers) throw Error('Compiler mismatch');
    return (await WebAssembly.instantiate(module)).exports;
  }));
}
function measure(name, target, run, iterations, round, check) {
  const samples = [[], []], checksums = new Set();
  for (let i = 0; i < 3; i++) for (const f of run) checksums.add(f().checksum);
  for (let sample = 0; sample < 7; sample++) {
    for (const i of (sample + round) % 2 ? [1, 0] : [0, 1]) {
      const result = run[i](); samples[i].push(result.ms); checksums.add(result.checksum);
    }
  }
  if (checksums.size !== 1 || (check !== undefined && [...checksums][0] !== check)) throw Error(`Checksum ${name}: ${[...checksums]}`);
  const medians = samples.map(s => s.toSorted((a,b) => a-b)[3]);
  const row = { name, target, iterations, round, checksum: [...checksums][0], samples_ms: samples, medians_ms: medians };
  rows.push(row); save();
  console.log(JSON.stringify({ name, target, round, ratio: medians[1] / medians[0] }));
}
function wasmRun(e, fn, args) {
  return () => { const start = performance.now(); const checksum = e[fn](...args) >>> 0; return { checksum, ms: performance.now() - start }; };
}
if (suite === 'fpu') {
  for (const [artifact, modes] of [['fpu_bench_x87', [0]], ['fpu_bench_flat_x87', [0,1]]]) {
    const entries = await exportsFor(artifact);
    for (const mode of modes) for (let c = 0; c < 4; c++) {
      const iterations = 256;
      const expected = entries.map(e => e.prepare_fpu(c, mode) >>> 0);
      if (expected[0] !== expected[1]) throw Error('FPU preparation checksum');
      const name = `${artifact}/mode${mode}/case${c}`;
      const wasm = entries.map(e => wasmRun(e, 'run_fpu', [iterations]));
      const native = dirs.map(dir => {
        const path = join(dir, artifact); hash(path);
        return () => {
          const [checksum, ms] = execFileSync(path, [String(c), String(iterations), String(mode)], { encoding: 'utf8' }).trim().split(/\s+/).map(Number);
          return { checksum, ms };
        };
      });
      for (let round = 0; round < 3; round++) {
        const checksum = Math.imul(expected[0], iterations) >>> 0;
        measure(name, 'native', native, iterations, round, checksum);
        measure(name, 'wasm', wasm, iterations, round, checksum);
      }
    }
  }
} else if (suite === 'frequency') {
  const manifestPath = new URL('../../../fixtures/frequency-v1/manifest.json', import.meta.url);
  const manifest = JSON.parse(readFileSync(manifestPath));
  metadata.manifestHash = createHash('sha256').update(readFileSync(manifestPath)).digest('hex');
  for (const [artifact, mode] of [['frequency_bench',0], ['frequency_bench_flat',0], ['flat_frequency_bench',1]]) {
    const entries = await exportsFor(artifact);
    if (entries.some(e => e.frequency_uop_size() !== 8)) throw Error('Unexpected Uop layout');
    for (const c of manifest.cases) {
      const expected = entries.map(e => (e.prepare_frequency_mode ? e.prepare_frequency_mode(c.id,mode) : e.prepare_frequency(c.id)) >>> 0);
      if (expected[0] !== expected[1]) throw Error('Frequency preparation checksum');
      const wasm = entries.map(e => wasmRun(e, 'run_frequency', [c.iterations]));
      const native = dirs.map(dir => {
        const path = join(dir,artifact); hash(path);
        return () => {
          const [checksum,ms] = execFileSync(path,[String(c.id),String(c.iterations),String(mode)],{encoding:'utf8'}).trim().split(/\s+/).map(Number);
          return {checksum,ms};
        };
      });
      for (let round = 0; round < 3; round++) {
        const checksum = Math.imul(expected[0],c.iterations) >>> 0;
        const name = `${artifact}/${c.name}`;
        measure(name,'native',native,c.iterations,round,checksum);
        measure(name,'wasm',wasm,c.iterations,round,checksum);
      }
    }
  }
} else if (suite === 'stress') {
  const entries = await exportsFor('flat_memory_stress');
  const functions = 4096, iterations = 4;
  metadata.functions = functions;
  metadata.cachedUops = entries.map(e => e.prepare_stress(functions));
  if (metadata.cachedUops[0] !== metadata.cachedUops[1]) throw Error('Stress cache layout');
  for (let mode = 0; mode < 3; mode++) {
    const native = dirs.map(dir => {
      const path = join(dir,'flat_memory_stress'); hash(path);
      return () => {
        const rows = execFileSync(path,[String(functions),String(mode),String(iterations)],{encoding:'utf8'}).trim().split('\n').map(line=>line.split(/\s+/).map(Number));
        if (new Set(rows.map(r=>r[2])).size !== 1) throw Error('Native stress checksum');
        return {checksum: rows[0][2], ms: rows.map(r=>r[0]).toSorted((a,b)=>a-b)[3]};
      };
    });
    const wasm = entries.map(e => wasmRun(e,'run_stress',[mode,iterations]));
    for (let round=0; round<3; round++) {
      const expected = functions*iterations + (mode===2 ? iterations/2 : 0);
      measure(`stress/mode${mode}`,'native',native,iterations,round,expected);
      measure(`stress/mode${mode}`,'wasm',wasm,iterations,round,expected);
    }
  }
} else if (suite === 'guards') {
  const cpu = await exportsFor('cpu_dispatch_bench');
  const strings = await exportsFor('string_bench');
  for (let round = 0; round < 3; round++) {
    measure('cpu-dispatch', 'wasm', cpu.map(e => wasmRun(e,'bench_cpu_dispatch',[256])),256,round);
    for (const [kind, width, reverse, scenario] of [[0,1,0,0],[0,4,0,2],[0,4,1,3],[1,2,0,0],[2,4,0,0],[3,1,0,0],[3,4,1,4],[4,2,0,0]]) {
      const args = [kind,width,65536,reverse,scenario,0,128];
      measure(`string/${kind}/${width}/${reverse}/${scenario}`,'wasm',strings.map(e => wasmRun(e,'bench_string',args)),128,round);
    }
  }
} else throw Error('Unknown suite');
