"""Verify measured artifact hashes, fixed inputs, and reference output checksums."""
import hashlib
import json
import sys
from pathlib import Path
repo=Path(__file__).resolve().parents[2];root=Path(sys.argv[1]);metadata=json.loads((root/'metadata.json').read_text())
manifest=(repo/'fixtures/frequency-v1/manifest.json').read_bytes()
manifest_hash=hashlib.sha256(manifest).hexdigest()
reference={c['case']:c['checksum'] for c in json.loads((root/'profiles/baseline/frequency.json').read_text())['cases']}
checked=[]
for p in sorted(root.glob('*.json')):
 parts=p.stem.split('-',1)
 if len(parts)!=2 or parts[0] not in ['native','browser','node']:continue
 data=json.loads(p.read_text())
 if 'rows' not in data:continue
 env,variant=parts
 assert data['metadata']['manifest_sha256']==manifest_hash,p
 target='native' if env=='native' else 'wasm'
 assert data['metadata']['hashes']==[metadata['artifact_hashes'][v][target] for v in ['baseline',variant]],p
 for row in data['rows']:
  assert row['checksum']==(reference[row['name']]*row['iterations'])&0xffffffff,(p,row['name'])
 checked.append(p.name)
(root/'verification.json').write_text(json.dumps(dict(checked=checked,artifact_hashes_match=True,reference_checksums_match=True,guest_counts_match=metadata['guest_counts_match']),indent=2)+'\n')
print('Artifacts, manifest and independent reference checksums verified:',len(checked))
