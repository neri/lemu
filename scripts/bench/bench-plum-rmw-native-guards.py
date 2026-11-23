"""Compare common-dispatch guards in two matching release snapshots."""
import json
import statistics
import subprocess
import sys
from pathlib import Path

long_run = "--long" in sys.argv
output, *roots = [arg for arg in sys.argv[1:] if arg != "--long"]
assert len(roots) == 2, 'Usage: python3 script output.json before-root after-root'
rows = []
for batch in range(3):
    for stage in ([0, 1] if batch % 2 == 0 else [1, 0]):
        binary = Path(roots[stage]) / 'target/release/examples'
        text = subprocess.check_output([str(binary / 'cpu_dispatch_bench')] + (['1024'] if long_run else []), text=True)
        samples = list(map(float, text.splitlines()))
        rows.append(dict(batch=batch, stage=stage, name='fib',
                         median_ms=statistics.median(samples), samples_ms=samples))
        text = subprocess.check_output([str(binary / 'return_dispatch_bench')] + (['256'] if long_run else []), text=True)
        for line in text.splitlines():
            name, elapsed = line.split()
            rows.append(dict(batch=batch, stage=stage,
                             name='return-' + name.split('=')[1],
                             median_ms=float(elapsed.split('=')[1])))
    print(f'batch {batch + 1}/3', flush=True)
metadata = dict(roots=roots, long_run=long_run, affinity=next(line for line in
                Path('/proc/self/status').read_text().splitlines()
                if line.startswith('Cpus_allowed_list:')))
Path(output).write_text(json.dumps(dict(metadata=metadata, rows=rows), indent=2))
for name in sorted({r['name'] for r in rows}):
    ratios = []
    for batch in range(3):
        times = {r['stage']: r['median_ms'] for r in rows
                 if r['name'] == name and r['batch'] == batch}
        ratios.append(100 * (times[1] / times[0] - 1))
    print(name, f'{statistics.median(ratios):+.2f}%', ratios)
