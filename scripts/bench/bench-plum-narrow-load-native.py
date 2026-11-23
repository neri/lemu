"""Alternate native A/B samples; binary timers exclude process startup/warmup."""
import json
import statistics
import subprocess
import sys
from pathlib import Path

screen = "--screen" in sys.argv
output, before, after = [s for s in sys.argv[1:] if s != "--screen"]
roots = [before, after]
binaries = [str(Path(r) / 'target/release/examples/narrow_load_bench') for r in roots]
workloads = [
    ('bd8',128), ('bd16',128), ('absolute8',128), ('absolute16',128),
    ('cmp8-bd',128), ('add16-bd',128), ('movzx8-bd',128), ('movsx16-bd',128),
    ('base8',128), ('base16',128), ('sib8',128), ('sib16',128),
    ('bd32',128), ('sib32',128), ('movzx8-reg',128), ('movsx16-reg',128),
    ('alu',128), ('rmw',128), ('fib',1024), ('ret-1',256), ('ret-4',64),
    ('ret-depth8',32), ('ret-16',16), ('stos',1024), ('scas',1024),
]
selected = [0,1,2,3,16,17,18,19] if screen else list(range(len(workloads)))
probes = [subprocess.check_output([b, 'probe'], text=True).strip() for b in binaries]
assert len(set(probes)) == 1, probes
assert int(probes[0].split()[0]) > 65000 and probes[0].split()[1:] == ['22412', '8', '4'], probes
rows = []
for batch in range(3):
    for offset in range(len(selected)):
        case = selected[(offset + batch * 5) % len(selected)]
        name, iterations = workloads[case]
        samples = [[], []]
        checksums = set()
        for sample in range(3 if screen else 9):
            for stage in ([1, 0] if (batch + sample) % 2 else [0, 1]):
                checksum, elapsed = subprocess.check_output(
                    [binaries[stage], str(case), str(iterations)], text=True).split()
                checksums.add(checksum)
                samples[stage].append(float(elapsed))
        assert len(checksums) == 1, (name, checksums)
        for stage in range(2):
            rows.append(dict(batch=batch, name=name, stage=stage, iterations=iterations,
                             checksum=int(next(iter(checksums))), samples_ms=samples[stage],
                             median_ms=statistics.median(samples[stage])))
        print(f'batch {batch + 1}/3 {name}', flush=True)
summary = []
for name, _ in (workloads[i] for i in selected):
    ratios = []
    for batch in range(3):
        times = {r['stage']: r['median_ms'] for r in rows if r['batch'] == batch and r['name'] == name}
        ratios.append(100 * (times[1] / times[0] - 1))
    summary.append(dict(name=name, change_percent=statistics.median(ratios), batches_percent=ratios))
metadata = dict(roots=roots, probes=probes, screen=screen, rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                affinity=next(s for s in Path('/proc/self/status').read_text().splitlines() if s.startswith('Cpus_allowed_list:')))
Path(output).write_text(json.dumps(dict(metadata=metadata, rows=rows, summary=summary), indent=2))
for row in summary:
    print(row['name'], f"{row['change_percent']:+.2f}%", row['batches_percent'])
