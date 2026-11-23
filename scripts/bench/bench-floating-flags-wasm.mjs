// Run on the same fixed CPU as the native comparison, without concurrent builds.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const [root, output, roundsArg='3'] = process.argv.slice(2);
if (!root || !output) throw Error('Usage: node bench-floating-flags-wasm.mjs SNAPSHOTS OUTPUT [ROUNDS]');
const integerControl=process.argv.includes('--integer-control');
const manifest = JSON.parse(readFileSync(new URL(integerControl ? '../../fixtures/frequency-v1/manifest.json' : '../../plum/examples/support/bench/floating-flags/guest/manifest.json',import.meta.url)));
const modes=integerControl?[1]:[1,0];
const variants=['baseline','cheap','deferred'];
const example=integerControl?'flat_frequency_bench':'floating_flags_bench';
const paths=variants.map(v=>`${root}/${v}/target/wasm32-unknown-unknown/release/examples/${example}.wasm`);
const bytes=paths.map(p=>readFileSync(p));
const modules=await Promise.all(bytes.map(b=>WebAssembly.compile(b)));
const entries=await Promise.all(modules.map(async m=>(await WebAssembly.instantiate(m)).exports));
const metadata={variants,paths,node:process.versions,hashes:bytes.map(b=>createHash('sha256').update(b).digest('hex')),sizes:bytes.map(b=>b.length),affinity:readFileSync('/proc/self/status','utf8').match(/^Cpus_allowed_list:\s*(.+)$/m)?.[1],profile:'release opt-level=3, lto=true, codegen-units=1, panic=abort',samples:7,warmups:3};
const rows=[];
for(let round=0;round<Number(roundsArg);round++) {
  for(const mode of modes) for(const c of manifest.cases) {
    const initial=entries.map(e=>[e.prepare_frequency_mode(c.id,mode)>>>0,(e.output_digest?.()??0)>>>0]);
    if(initial.some(x=>x[0]!==initial[0][0]||x[1]!==initial[0][1])) throw Error(`Initial checksum: ${c.name}/${mode}`);
    let iterations=4;
    for(;;) {
      const e=entries[0];
      for(let k=0;k<3;k++) e.run_frequency(iterations);
      const start=performance.now();e.run_frequency(iterations);
      if(performance.now()-start>=12||iterations>=256)break;
      iterations*=2;
    }
    const expected=Math.imul(initial[0][0],iterations)>>>0;
    for(const e of entries) for(let k=0;k<3;k++) if((e.run_frequency(iterations)>>>0)!==expected)throw Error('warmup');
    const samples=variants.map(()=>[]);
    for(let sample=0;sample<7;sample++) {
      const shift=(sample+round)%3;
      let order=[shift,(shift+1)%3,(shift+2)%3];
      if((sample+round)%2)order.reverse();
      for(const stage of order) {
        const e=entries[stage];
        const start=performance.now();const checksum=e.run_frequency(iterations)>>>0;
        samples[stage].push(performance.now()-start);
        if(checksum!==expected||((e.output_digest?.()??0)>>>0)!==initial[0][1])throw Error(`Measured checksum: ${c.name}/${mode}`);
      }
    }
    const medians=samples.map(s=>s.toSorted((a,b)=>a-b)[3]);
    rows.push({round,mode,case:c.id,name:c.name,backend:c.backend??'integer',family:c.family,iterations,samples_ms:samples,medians_ms:medians,checksum:expected,output_digest:initial[0][1]});
    writeFileSync(output,JSON.stringify({metadata,rows},null,2)+'\n');
    console.log(JSON.stringify({round,mode,name:c.name,ratios:medians.map(m=>m/medians[0])}));
  }
}
