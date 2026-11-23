"""Sequential independent-process A/B; timers exclude setup and three warmups."""
import argparse
import datetime
import hashlib
import json
import os
import platform
import statistics
import subprocess
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('output',type=Path);p.add_argument('baseline',type=Path);p.add_argument('candidate',type=Path)
p.add_argument('--screen',action='store_true')
p.add_argument('--scale',type=int,default=1)
a=p.parse_args()
assert a.scale>=1
repo=Path(__file__).resolve().parents[2]
manifest_path=repo/'fixtures/frequency-v1/manifest.json'
manifest=json.loads(manifest_path.read_text())
roots=[a.baseline.resolve(),a.candidate.resolve()]
for r in roots:
 assert (r/'fixtures/frequency-v1/manifest.json').read_bytes()==manifest_path.read_bytes(), 'Snapshot manifest mismatch'
binaries=[r/'target/release/examples/frequency_bench' for r in roots]
metadata=dict(date=datetime.datetime.now(datetime.timezone.utc).isoformat(),environment='native',screen=a.screen,roots=list(map(str,roots)),
              hashes=[hashlib.sha256(b.read_bytes()).hexdigest() for b in binaries],
              manifest_sha256=hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
              rustc=subprocess.check_output(['rustc','-Vv'],text=True),cpu=next((s.split(':',1)[1].strip() for s in Path('/proc/cpuinfo').read_text().splitlines() if s.startswith('model name')),platform.processor()),
              affinity=sorted(os.sched_getaffinity(0)), counter_enabled=False,iteration_scale=a.scale)
rows=[]
for batch in range(3 if a.screen else manifest['batches']):
 for offset in range(len(manifest['cases'])):
  case=manifest['cases'][(offset+batch*5)%len(manifest['cases'])]
  samples=[[],[]];checksums=set()
  for sample in range(3 if a.screen else manifest['samples']):
   for stage in ([1,0] if (batch+sample)%2 else [0,1]):
    checksum,elapsed=subprocess.check_output([str(binaries[stage]),str(case['id']),str(case['iterations']*a.scale)],text=True).split()
    checksums.add(int(checksum));samples[stage].append(float(elapsed))
  assert len(checksums)==1,(case['name'],checksums)
  for stage in range(2):
   rows.append(dict(batch=batch,case=case['id'],name=case['name'],stage=stage,iterations=case['iterations']*a.scale,
                    checksum=next(iter(checksums)),samples_ms=samples[stage],median_ms=statistics.median(samples[stage])))
  a.output.write_text(json.dumps(dict(metadata=metadata,rows=rows),indent=2)+'\n')
 print(f'{a.candidate.name}: batch {batch+1} complete',flush=True)
