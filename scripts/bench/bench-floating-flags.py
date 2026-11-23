"""Alternate isolated candidates on compiler-generated mixed floating workloads."""
import argparse
import hashlib
import json
import os
import statistics
import subprocess
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('snapshots',type=Path)
p.add_argument('output',type=Path)
p.add_argument('--rounds',type=int,default=3)
p.add_argument('--iterations',type=int,default=0,help='0 calibrates to at least 12ms per baseline sample')
p.add_argument('--variants',default='baseline,cheap,deferred')
p.add_argument('--integer-control',action='store_true')
a=p.parse_args()
repo=Path(__file__).resolve().parents[2]
variants=a.variants.split(',')
example='flat_frequency_bench' if a.integer_control else 'floating_flags_bench'
paths=[a.snapshots.resolve()/v/'target/release/examples'/example for v in variants]
manifest=json.loads((repo/('fixtures/frequency-v1/manifest.json' if a.integer_control else 'plum/examples/support/bench/floating-flags/guest/manifest.json')).read_text())
modes=[1] if a.integer_control else [1,0]
os.sched_setaffinity(0,{min(os.sched_getaffinity(0))})
metadata=dict(variants=variants,paths=list(map(str,paths)),hashes=[hashlib.sha256(p.read_bytes()).hexdigest() for p in paths],affinity=sorted(os.sched_getaffinity(0)),iterations=a.iterations,rounds=a.rounds,profile='release opt-level=3, lto=true, codegen-units=1, panic=abort',samples=7,warmups=3)
iterations={}
for mode in modes:
    for c in manifest['cases']:
        n=a.iterations or 16
        if not a.iterations:
            while True:
                result=subprocess.check_output([str(paths[0]),str(c['id']),str(n),str(mode)],text=True).split()
                if float(result[-1]) >= 12 or n >= 256: break
                n*=2
        iterations[(mode,c['id'])]=n
metadata['case_iterations']={f'{mode}:{case}':n for (mode,case),n in iterations.items()}
rows=[]
for round in range(a.rounds):
    for mode in modes:
        for c in manifest['cases']:
            samples=[[] for _ in variants];checksums=set()
            n=iterations[(mode,c['id'])]
            for sample in range(7):
                shift=(sample+round)%len(variants)
                order=list(range(len(variants)))
                order=order[shift:]+order[:shift]
                if (sample+round)%2: order.reverse()
                for stage in order:
                    parts=subprocess.check_output([str(paths[stage]),str(c['id']),str(n),str(mode)],text=True).split()
                    checksums.add(tuple(map(int,parts[:-1])));samples[stage].append(float(parts[-1]))
            assert len(checksums)==1,(c['name'],mode,checksums)
            medians=list(map(statistics.median,samples))
            row=dict(iterations=n,round=round,mode=mode,case=c['id'],name=c['name'],backend=c.get('backend','integer'),family=c['family'],samples_ms=samples,medians_ms=medians,checksum=next(iter(checksums)))
            rows.append(row)
            a.output.write_text(json.dumps(dict(metadata=metadata,rows=rows),indent=2)+'\n')
            print(json.dumps(dict(round=round,mode=mode,name=c['name'],ratios=[m/medians[0] for m in medians])),flush=True)
