"""Alternate native A/B samples; binary timers exclude process startup/warmup."""
import json
import statistics
import subprocess
import sys
from pathlib import Path

output, before, after = sys.argv[1:]
roots = [before, after]
binaries = [str(Path(r) / 'target/release/examples/load_extend_bench') for r in roots]
workloads = [
    ('zx8', 128), ('zx16', 128), ('sx8', 128), ('sx16', 128), ('mixed', 128),
    ('alu', 128), ('rmw', 128), ('fib', 1024), ('ret-1', 256), ('ret-4', 64),
    ('ret-depth8', 32), ('ret-16', 16), ('stos', 1024), ('scas', 1024),
]
probes = [subprocess.check_output([b, 'probe'], text=True).strip() for b in binaries]
assert probes == ['22412 8 4'] * 2, probes
rows = []
for batch in range(3):
    for offset in range(len(workloads)):
        case = (offset + batch * 5) % len(workloads)
        name, iterations = workloads[case]
        samples = [[], []]
        checksums = set()
        for sample in range(9):
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
for name, _ in workloads:
    ratios = []
    for batch in range(3):
        times = {r['stage']: r['median_ms'] for r in rows if r['batch'] == batch and r['name'] == name}
        ratios.append(100 * (times[1] / times[0] - 1))
    summary.append(dict(name=name, change_percent=statistics.median(ratios), batches_percent=ratios))
metadata = dict(roots=roots, probes=probes, rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                affinity=next(s for s in Path('/proc/self/status').read_text().splitlines() if s.startswith('Cpus_allowed_list:')))
Path(output).write_text(json.dumps(dict(metadata=metadata, rows=rows, summary=summary), indent=2))
for row in summary:
    print(row['name'], f"{row['change_percent']:+.2f}%", row['batches_percent'])
