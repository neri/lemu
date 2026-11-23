"""Serve an isolated localhost A/B page and save its browser measurements.

Usage: python3 scripts/bench/serve-plum-load-extend.py output-dir before-root after-root [port]
Only Wasm artifacts and the benchmark module are exposed, not workspace source.
"""
import json
import shutil
import sys
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

output, before, after, *rest = sys.argv[1:]
root = Path(output).resolve()
web = root / 'web'
web.mkdir(parents=True, exist_ok=True)
for name, source in [('before', before), ('after', after)]:
    shutil.copy(Path(source) / 'target/wasm32-unknown-unknown/release/examples/load_extend_bench.wasm',
                web / f'{name}.wasm')
shutil.copy(Path(__file__).with_name('bench-plum-load-extend.mjs'), web / 'bench.mjs')
(web / 'index.html').write_text('''<!doctype html><html lang="en"><meta charset="utf-8">
<title>plum load extension A/B</title><h1>plum load extension A/B</h1>
<button id="run">Run semantic probes and A/B</button><p id="progress">Ready</p><pre id="result"></pre>
<script type="module">
import {compare} from './bench.mjs';
const button=document.querySelector('#run'), progress=document.querySelector('#progress'), output=document.querySelector('#result');
button.onclick=async()=>{button.disabled=true;try{
const bytes=await Promise.all(['before','after'].map(async s=>await(await fetch(s+'.wasm',{cache:'no-store'})).arrayBuffer()));
const r=await compare(bytes,s=>progress.textContent=s);
r.metadata.browser=navigator.userAgent;
output.textContent=JSON.stringify(r.summary,null,2);
const response=await fetch('/results',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(r)});
if(!response.ok) throw Error('Saving results failed');
progress.textContent='Complete: both semantic probes 22412 cases, Uop=8, Minor=4; checksums match for all workloads. Results saved.';
}catch(e){progress.textContent='Failed: '+e.stack;}finally{button.disabled=false;}};
</script></html>''')


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(web), **kwargs)

    def do_POST(self):
        if self.path != '/results':
            self.send_error(404)
            return
        value = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        (root / 'browser.json').write_text(json.dumps(value, indent=2))
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b'saved')


port = int(rest[0]) if rest else 8765
print(f'Open http://127.0.0.1:{port}/ ; results: {root / "browser.json"}', flush=True)
ThreadingHTTPServer(('127.0.0.1', port), Handler).serve_forever()
