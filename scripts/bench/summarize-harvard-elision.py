"""Report paired batch B/A means and two-sided Student t intervals."""
import csv
import json
import statistics as st
import sys
from collections import defaultdict
from pathlib import Path

root = Path(__file__).resolve().parents[2]
manifest = json.loads((root / 'fixtures/frequency-v1/manifest.json').read_text())
weights = {c['id']: c['case_weight'] * c['family_weight'] for c in manifest['cases']}
assert abs(sum(weights.values()) - 1) < 1e-8
ratio = defaultdict(dict)
for row in csv.DictReader(open(sys.argv[1])):
    stage = int(row['n'])
    a = int(row['a_ns'])
    if a <= 0:
        raise ValueError('timer resolution too low')
    ratio[int(row['case']), stage][int(row['batch'])] = int(row['b_ns']) / a
for n in [0] + [1 << i for i in range(11)]:
    ratio[-1, n] = {batch: sum(weights[c] * ratio[c, n][batch] for c in weights)
                    for batch in ratio[0, n]}
writer = csv.writer(sys.stdout, lineterminator='\n')
writer.writerow(['case','n','mean','lower95','upper95'])
for (case,n), rows in sorted(ratio.items()):
    values = rows.values()
    mean = st.mean(values)
    t = {5:2.77645, 10:2.26216, 20:2.09302}[len(values)]
    margin = t * st.stdev(values) / len(values)**0.5
    writer.writerow([case,n,*[round(x,6) for x in (mean,mean-margin,mean+margin)]])
