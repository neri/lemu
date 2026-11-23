"""Tabulate visited candidate sites from analysis-only and rewriting profile builds."""
import csv
import json
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
corpus = root / 'fixtures/frequency-v1'
names = [row['name'] for row in json.loads((corpus / 'manifest.json').read_text())['cases']]
writer = csv.writer(sys.stdout, lineterminator='\n')
writer.writerow(['case', 'ir', 'sites', 'visits', 'applied', 'original_pair_sites', 'original_pair_visits'])
for case in range(16):
    profiles = []
    for binary in sys.argv[1:3]:
        output = subprocess.check_output([binary, str(case), '1'], text=True)
        checksum, counters, *table = output.splitlines()
        assert 'faults=0 ' in counters
        profiles.append((checksum, {int(r['eip']): r for r in csv.DictReader(table, delimiter='\t')}))
    (checksum_a, analysis), (checksum_b, candidate) = profiles
    assert checksum_a == checksum_b and analysis.keys() == candidate.keys()
    counts = {}
    code = bytes.fromhex((corpus / 'guest' / (names[case]+'.bin.hex')).read_text()) if case < 14 else None
    for eip, row in analysis.items():
        other = candidate[eip]
        assert row['entries'] == other['entries'] and row['ir'] == other['ir']
        if not row['ir'].endswith(' [candidate]'):
            continue
        form = row['ir'].split('(')[0]
        visits = int(row['entries'])
        bucket = counts.setdefault(form, [0]*5)
        bucket[0] += 1
        bucket[1] += visits
        if other['uops'] != row['uops']:
            bucket[2] += visits
        if form == 'ADD_Rd_Id':
            # The fixed guests use unprefixed accumulator/ModRM immediate forms.
            length = {0x05:5, 0x81:6, 0x83:3}[code[eip-0x100000]] if code else 3
            following = analysis.get(eip + length)
            if following and following['ir'].startswith('CMP_Rd_Id('):
                bucket[3] += 1
                bucket[4] += visits
    for form, counts_for_form in sorted((counts or {'':[0]*5}).items()):
        writer.writerow([case, form, *counts_for_form])
