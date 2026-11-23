"""Write per-case and per-family ratios, retaining regressions alongside gains."""
import gzip
import json
import math
import statistics
import sys
from pathlib import Path

groups = [('binary', 0, 42), ('unary', 42, 54), ('shift', 54, 84), ('carry_rotate', 84, 96), ('double_shift', 96, 104), ('bit_update', 104, 116), ('xchg', 116, 119)]
for filename in sys.argv[1:]:
    path = Path(filename)
    data = json.loads(gzip.decompress(path.read_bytes()) if path.suffix == '.gz' else path.read_text())
    rows = {}
    for row in data['rows']:
        rows.setdefault((row['batch'], row['name']), {})[row['stage']] = row['median_ms']
    summary = ['name,delta_percent,batch0_percent,batch1_percent,batch2_percent']
    for name in sorted({name for _, name in rows}):
        values = [100 * (rows[(batch, name)][1] / rows[(batch, name)][0] - 1) for batch in range(3)]
        summary.append(f'{name},{statistics.median(values):.6f},' + ','.join(f'{v:.6f}' for v in values))
    for group, start, end in groups:
        values = []
        for batch in range(3):
            ratios = [v[1] / v[0] for (b, name), v in rows.items() if b == batch and name.startswith('rmw-') and start <= int(name.split('-')[1]) < end]
            values.append(100 * (math.exp(statistics.mean(map(math.log, ratios))) - 1))
        summary.append(f'group-{group},{statistics.median(values):.6f},' + ','.join(f'{v:.6f}' for v in values))
        print(path.name, group, f'{statistics.median(values):+.2f}%')
    path.with_name(path.name.removesuffix('.gz').removesuffix('.json') + '-summary.csv').write_text('\n'.join(summary) + '\n')
