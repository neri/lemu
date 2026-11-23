"""Compare full profiling TSV output, including headers, counts, and row order."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

before, after, output = map(Path, sys.argv[1:])
rows = []
for artifact, mode in [("frequency_bench_profile", 0), ("flat_frequency_bench_profile", 1)]:
    for case in range(14):
        reports = [subprocess.check_output([str((directory / artifact).resolve()), str(case), "1", str(mode)]) for directory in [before, after]]
        assert reports[0] == reports[1], (artifact, case)
        rows.append(dict(artifact=artifact, case=case, identical=True, sha256=hashlib.sha256(reports[0]).hexdigest()))
output.write_text(json.dumps(dict(rows=rows), indent=2) + "\n")
print(f"{len(rows)} complete profile reports match")
