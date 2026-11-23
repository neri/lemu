"""Compare matching release example binaries; sample in alternating process order."""
import ast
import json
import platform
import subprocess
import sys
from pathlib import Path

selected = next((arg.split('=', 1)[1] for arg in sys.argv if arg.startswith('--cases=')), None)
selected_cases = set(map(int, selected.split(','))) if selected else None
output, *roots = [arg for arg in sys.argv[1:] if not arg.startswith('--')]
assert len(roots) >= 2, 'Usage: python3 script output.json before-root after-root [other-root] [--cases=0,1]'
rows = []
metadata = dict(roots=roots, selected_cases=sorted(selected_cases) if selected_cases else None, python=platform.python_version(), affinity=next(line.strip() for line in Path('/proc/self/status').read_text().splitlines() if line.startswith('Cpus_allowed_list:')))
for batch in range(3):
    for offset in range(119):
        case = (offset + batch * 40) % 119
        if selected_cases is not None and case not in selected_cases:
            continue
        checksums = {}
        order = list(range(len(roots)))
        if (offset + batch) % 2:
            order.reverse()
        for stage in order:
            text = subprocess.check_output([str(Path(roots[stage]) / 'target/release/examples/rmw_dispatch_bench'), str(case)], text=True)
            for line in text.splitlines():
                kind, scenario, checksum, median, samples = line.split(',', 4)
                key = int(scenario)
                value = int(checksum)
                if key in checksums:
                    assert checksums[key] == value, (case, scenario, stage)
                checksums[key] = value
                rows.append(dict(batch=batch, name=f'rmw-{kind}-{scenario}', stage=stage, checksum=value, median_ms=float(median), samples_ms=ast.literal_eval(samples)))
        if offset % 30 == 0:
            print(f'batch {batch + 1}/3: {offset}/119', flush=True)
    Path(output).write_text(json.dumps(dict(metadata=metadata, rows=rows), indent=2))
print(f'Saved {len(rows)} rows to {output}')
