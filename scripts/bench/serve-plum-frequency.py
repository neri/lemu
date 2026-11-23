"""Serve isolated artifacts for actual-browser sequential multi-candidate A/B."""
import json
import shutil
import sys
from http.server import SimpleHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
root=Path(sys.argv[1]).resolve();web=root/'web';web.mkdir(exist_ok=True)
variants=sys.argv[2:]
for name in variants:
 assert (root/name/'fixtures/frequency-v1/manifest.json').read_bytes()==(repo/'fixtures/frequency-v1/manifest.json').read_bytes(), "Snapshot manifest mismatch"
 shutil.copy(root/name/'target/wasm32-unknown-unknown/release/examples/frequency_bench.wasm',web/f'{name}.wasm')
shutil.copy(repo/'scripts/bench/bench-plum-frequency.mjs',web/'bench.mjs')
shutil.copy(repo/'fixtures/frequency-v1/manifest.json',web/'manifest.json')
(web/'index.html').write_text('''<!doctype html><meta charset="utf-8"><title>plum frequency review</title>
<button id="run">Run A/B</button><pre id="progress">Ready</pre><pre id="result"></pre>
<script type="module">
import {compare} from './bench.mjs';
const variants=VARIANTS;
window.results=[];
document.querySelector('#run').onclick=async()=>{
 document.querySelector('#run').disabled=true;
 try {
 const manifest=await(await fetch('manifest.json')).arrayBuffer();
 const base=await(await fetch('baseline.wasm')).arrayBuffer();
 const params=new URLSearchParams(location.search);
 const selected=params.get('variants')?.split(',')??variants.filter(x=>x!=='baseline');
 if(selected.some(x=>!variants.includes(x)||x==='baseline')) throw Error('Invalid selection');
 for(const name of selected) {
 const after=await(await fetch(name+'.wasm')).arrayBuffer();
 const result=await compare([base,after],manifest,s=>document.querySelector('#progress').textContent=name+': '+s,false,Number(params.get('scale')??1));
 result.metadata.variant=name;
 const response=await fetch('/results/'+name,{method:'POST',body:JSON.stringify(result)});
 if(!response.ok) throw Error('Save failed');
 window.results.push(name);document.querySelector('#result').textContent=window.results.join(' complete; ');
 }
 document.querySelector('#progress').textContent='Complete';
 } catch(e) {document.querySelector('#progress').textContent='Failed: '+e.stack;}
};
</script>'''.replace('VARIANTS',json.dumps(variants)))
class Handler(SimpleHTTPRequestHandler):
 def __init__(self,*args,**kwargs):super().__init__(*args,directory=str(web),**kwargs)
 def do_POST(self):
  name=self.path.removeprefix('/results/')
  if not self.path.startswith('/results/') or name not in variants:
   self.send_error(404);return
  value=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
  (root/f'browser-{name}.json').write_text(json.dumps(value,indent=2)+'\n')
  self.send_response(200);self.end_headers();self.wfile.write(b'saved')
print('http://127.0.0.1:8765/',flush=True)
ThreadingHTTPServer(('127.0.0.1',8765),Handler).serve_forever()
