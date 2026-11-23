"""Run the paired series in a local headless Firefox, serving two trusted Wasm builds."""
import argparse
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import subprocess
import tempfile
import secrets

parser = argparse.ArgumentParser()
parser.add_argument("baseline", type=Path)
parser.add_argument("candidate", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
modules = {"/A": args.baseline.read_bytes(), "/B": args.candidate.read_bytes()}
token = secrets.token_hex(16)
page = r"""<body><script type=module>
try {
  const modules = await Promise.all(['/A','/B'].map(async path =>
    (await WebAssembly.instantiateStreaming(fetch(path))).instance.exports));
  const rows=['batch,case,n,a_ns,b_ns,eax_sum'];
  for(let batch=0;batch<20;batch++) {
    for(let offset=0;offset<16;offset++) {
      const c=(offset+5*batch)%16;
      const samples=[[],[]];
      for(const stage of (batch+c)%2?[1,0]:[0,1]) {
        const e=modules[stage];
        for(const n of [1,2,4,8,16,32,64,128,256,512,1024,0]) {
          e.setup_series(c);
          if (!n) {e.guest_series(4);e.resume_series();}
          const start=performance.now();
          const sum=e.guest_series(n||128)>>>0;
          samples[stage].push([Math.round((performance.now()-start)*1e6),sum,n]);
        }
      }
      await new Promise(resolve=>setTimeout(resolve,0));
      for(let i=0;i<12;i++) {
        const [a,sum,n]=samples[0][i], [b,other]=samples[1][i];
        if(sum!==other)throw Error('checksum mismatch '+c);
        rows.push([batch,c,n,a,b,sum].join(','));
      }
    }
  }
  await fetch('/TOKEN',{method:'POST',body:rows.join('\n')+'\n'});
} catch(err) {await fetch('/TOKEN',{method:'POST',body:String(err)});}
</script>""".replace("TOKEN", token).encode()

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path not in modules and self.path != "/":
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/html" if self.path == "/" else "application/wasm")
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.end_headers()
        self.wfile.write(modules.get(self.path, page))

    def do_POST(self):
        if self.path != "/" + token:
            self.send_error(404)
            return
        result = self.rfile.read(int(self.headers["Content-Length"]))
        self.server.result = result
        self.send_response(200)
        self.end_headers()

with HTTPServer(("127.0.0.1", 0), Handler) as server, tempfile.TemporaryDirectory() as profile:
    url = f"http://127.0.0.1:{server.server_port}/"
    browser = subprocess.Popen(["firefox", "--headless", "--no-remote", "--profile", profile, url])
    try:
        server.timeout = 2
        while not hasattr(server, "result"):
            if browser.poll() is not None:
                raise RuntimeError("Firefox exited before completing the benchmark")
            server.handle_request()
        if not server.result.startswith(b"batch,"):
            raise RuntimeError(server.result.decode())
        args.output.write_bytes(server.result)
    finally:
        browser.terminate()
        browser.wait(timeout=30)
