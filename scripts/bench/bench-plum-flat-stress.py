import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess

binary = Path('target/release/examples/flat_memory_stress').resolve()
os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
rows = []
for functions in [16, 256, 4096]:
    iterations = 16384 // functions
    for mode in range(3):
        samples = [line.split() for line in subprocess.check_output([str(binary), str(functions), str(mode), str(iterations)], text=True).splitlines()]
        checksums = {int(s[2]) for s in samples}
        expected = functions * iterations + (iterations // 2 if mode == 2 else 0)
        assert checksums == {expected}, (functions, mode, checksums)
        row = dict(functions=functions, mode=mode, iterations=iterations, cached_uops=int(samples[0][1]), checksum=expected, samples_ms=[float(s[0]) for s in samples])
        row['median_ms'] = statistics.median(row['samples_ms'])
        rows.append(row)
        print(row, flush=True)
output = Path('target/bench/plum-flat-stress/native.json')
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(dict(artifact=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), affinity=sorted(os.sched_getaffinity(0)), rows=rows), indent=2) + '\n')
