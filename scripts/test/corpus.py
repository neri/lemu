"""Validate the frozen guest bytes used as native/Wasm build inputs."""
import hashlib
import json
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
root = repo / 'fixtures/frequency-v1'
cases = json.loads((root / 'manifest.json').read_text())['cases']
for case in cases:
    guest = root / 'guest' / (case['name'] + '.bin.hex')
    actual = hashlib.sha256(bytes.fromhex(guest.read_text())).hexdigest()
    if actual != case['code_sha256']:
        raise SystemExit(f"Guest hash mismatch: {guest.name}: {actual}")
print(f'{len(cases)} frozen guest hashes match the manifest')

root = repo / 'plum/examples/support/bench/floating-flags/guest'
cases = json.loads((root / 'manifest.json').read_text())['cases']
for case in cases:
    guest = root / (case['name'] + '.bin.hex')
    actual = hashlib.sha256(bytes.fromhex(guest.read_text())).hexdigest()
    if actual != case['sha256']:
        raise SystemExit(f"Guest hash mismatch: {guest.name}: {actual}")
print(f'{len(cases)} floating-point guest hashes match the manifest')

cases = json.loads((repo / 'fixtures/guest-manifest.json').read_text())['cases']
for case in cases:
    guest = repo / case['path']
    actual = hashlib.sha256(bytes.fromhex(guest.read_text())).hexdigest()
    if actual != case['sha256']:
        raise SystemExit(f"Guest hash mismatch: {guest.name}: {actual}")
print(f'{len(cases)} example/ELF guest hashes match the manifest')
