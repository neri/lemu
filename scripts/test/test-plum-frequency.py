"""Check fixed-family scoring, batch uncertainty, and screen rejection."""
import json
import subprocess
import tempfile
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
m=json.loads((repo/'fixtures/frequency-v1/manifest.json').read_text())
assert len({c['family'] for c in m['cases']})==5
for f in {c['family'] for c in m['cases']}:
 assert abs(sum(c['case_weight'] for c in m['cases'] if c['family']==f)-1)<1e-12
with tempfile.TemporaryDirectory() as d:
 root=Path(d)
 def write(env,screen=False,noisy=False):
  rows=[]
  for b in range(5):
   for c in m['cases']:
    # Six updates cases must still contribute just 20% of the score.
    ratio=.9 if c['family']=='updates' else 1
    if noisy:ratio += [-.1,.1,-.1,.1,0][b]
    for s in [0,1]:rows.append(dict(batch=b,case=c['id'],stage=s,median_ms=10*(ratio if s else 1),samples_ms=[10]*9))
  (root/f'{env}-fixture.json').write_text(json.dumps(dict(metadata=dict(screen=screen),rows=rows)))
 write('native');write('browser')
 subprocess.run(['python3',str(repo/'scripts/bench/summarize-plum-frequency.py'),str(root)],check=True,stdout=subprocess.DEVNULL)
 x=json.loads((root/'summary.json').read_text())['fixture']
 assert abs(x['native']['overall']['ratio']-.98)<1e-12
 assert x['decision']['eligible_by_timing']
 write('native',True);write('browser',True)
 subprocess.run(['python3',str(repo/'scripts/bench/summarize-plum-frequency.py'),str(root)],check=True,stdout=subprocess.DEVNULL)
 assert not json.loads((root/'summary.json').read_text())['fixture']['decision']['eligible_by_timing']
 write('native',noisy=True);write('browser',noisy=True)
 subprocess.run(['python3',str(repo/'scripts/bench/summarize-plum-frequency.py'),str(root)],check=True,stdout=subprocess.DEVNULL)
 assert not json.loads((root/'summary.json').read_text())['fixture']['decision']['eligible_by_timing']
print('Fixed weights, screen rejection and batch uncertainty: OK')
