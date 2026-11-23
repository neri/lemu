// Shared actual-browser/Node timing driver. Instances are new for each batch.
const median = xs => [...xs].sort((a,b)=>a-b)[Math.floor(xs.length/2)];
const hash = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),x=>x.toString(16).padStart(2,'0')).join('');
export async function compare(bytes, manifestBytes, report=()=>{}, screen=false, iterationScale=1) {
  if(!Number.isInteger(iterationScale)||iterationScale<1) throw Error('Invalid iteration scale');
  const manifest=JSON.parse(new TextDecoder().decode(manifestBytes));
  const modules=await Promise.all(bytes.map(b=>WebAssembly.compile(b)));
  const metadata={environment:typeof window==='undefined'?'node':'browser', screen, counter_enabled:false, iteration_scale:iterationScale,
    hashes:await Promise.all(bytes.map(hash)),manifest_sha256:await hash(manifestBytes),
    producers:modules.map(m=>WebAssembly.Module.customSections(m,'producers').map(s=>new TextDecoder().decode(s)).join('')),
    runtime:typeof navigator==='undefined'?process.version:navigator.userAgent, date:new Date().toISOString()};
  if (!metadata.producers[0] || new Set(metadata.producers).size!==1) throw Error('Compiler mismatch');
  const rows=[];
  for(let batch=0;batch<(screen?3:manifest.batches);batch++) {
    const entries=[];
    for(const stage of batch%2?[1,0]:[0,1]) entries[stage]=(await WebAssembly.instantiate(modules[stage])).exports;
    if(entries.some(e=>e.frequency_uop_size()!==8)) throw Error('Uop size');
    for(let offset=0;offset<manifest.cases.length;offset++) {
      const c=manifest.cases[(offset+batch*5)%manifest.cases.length];
      const iterations=c.iterations*iterationScale;
      const cold=[];const prepared=[];
      for(const stage of batch%2?[1,0]:[0,1]) {
        const start=performance.now();prepared[stage]=entries[stage].prepare_frequency(c.id)>>>0;
        cold[stage]=performance.now()-start;
      }
      const expected=prepared[0];
      if(prepared[1]!==expected) throw Error(`Prepare checksum ${c.name}`);
      const checksum=Math.imul(expected,iterations)>>>0;
      for(let warm=0;warm<manifest.warmups;warm++) for(const stage of batch%2?[1,0]:[0,1])
        if((entries[stage].run_frequency(iterations)>>>0)!==checksum) throw Error(`Warm checksum ${c.name}`);
      const samples=[[],[]];
      for(let sample=0;sample<(screen?3:manifest.samples);sample++) for(const stage of (batch+sample)%2?[1,0]:[0,1]) {
        const start=performance.now();
        const actual=entries[stage].run_frequency(iterations)>>>0;
        const elapsed=performance.now()-start;
        if(actual!==checksum) throw Error(`Sample checksum ${c.name}`);
        samples[stage].push(elapsed);
      }
      for(let stage=0;stage<2;stage++) rows.push({batch,case:c.id,name:c.name,stage,iterations,checksum,
        samples_ms:samples[stage],median_ms:median(samples[stage]),cold_prepare_ms:cold[stage]});
      report(`batch ${batch+1}: ${c.name}`);
      await new Promise(resolve=>setTimeout(resolve,0));
    }
  }
  return {metadata,rows};
}
if(typeof process!=='undefined' && process.argv[1]?.endsWith('bench-plum-frequency.mjs')) {
 const {readFileSync,writeFileSync}=await import('node:fs');
 const [output,before,after]=process.argv.slice(2).filter(x=>x!=='--screen');
 const roots=[before,after];
 for(const r of roots) if(!readFileSync(`${r}/fixtures/frequency-v1/manifest.json`).equals(readFileSync(new URL('../fixtures/frequency-v1/manifest.json',import.meta.url)))) throw Error('Snapshot manifest mismatch');
 const result=await compare(roots.map(r=>readFileSync(`${r}/target/wasm32-unknown-unknown/release/examples/frequency_bench.wasm`)),
   readFileSync(new URL('../fixtures/frequency-v1/manifest.json',import.meta.url)),console.log,process.argv.includes('--screen'),Number(process.argv.find(x=>x.startsWith('--scale='))?.slice(8)??1));
 result.metadata.roots=roots;
 writeFileSync(output,JSON.stringify(result,null,2)+'\n');
}
