"""Alternate compiler-generated guests across two revisions or memory modes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess

p = argparse.ArgumentParser()
p.add_argument('before', type=Path)
p.add_argument('after', type=Path)
p.add_argument('output', type=Path)
p.add_argument('--compare-modes', action='store_true')
p.add_argument('--mode', type=int, choices=[0, 1], default=1)
p.add_argument('--rounds', type=int, default=3)
p.add_argument('--cases', type=lambda s: list(map(int, s.split(','))))
a = p.parse_args()
paths = [a.before.resolve(), a.after.resolve()]
modes = [0, 1] if a.compare_modes else [a.mode, a.mode]
manifest = json.loads((Path(__file__).resolve().parents[2] / 'fixtures/frequency-v1/manifest.json').read_text())
os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
rows = []
metadata = dict(paths=list(map(str, paths)), modes=modes, hashes=[hashlib.sha256(p.read_bytes()).hexdigest() for p in paths], affinity=sorted(os.sched_getaffinity(0)), rustc=subprocess.check_output(['rustc', '-Vv'], text=True))
for round in range(a.rounds):
    for c in manifest['cases']:
        if a.cases and c['id'] not in a.cases:
            continue
        samples = [[], []]
        checksums = set()
        for sample in range(7):
            for stage in ([1, 0] if (sample + round) % 2 else [0, 1]):
                checksum, elapsed = subprocess.check_output([str(paths[stage]), str(c['id']), str(c['iterations']), str(modes[stage])], text=True).split()
                checksums.add(int(checksum))
                samples[stage].append(float(elapsed))
        assert len(checksums) == 1, (c['name'], checksums)
        medians = list(map(statistics.median, samples))
        rows.append(dict(round=round, case=c['id'], name=c['name'], iterations=c['iterations'], checksum=next(iter(checksums)), samples_ms=samples, medians_ms=medians))
        a.output.write_text(json.dumps(dict(metadata=metadata, rows=rows), indent=2) + '\n')
        print(json.dumps(dict(round=round, name=c['name'], medians_ms=medians, ratio=medians[1] / medians[0])), flush=True)
