"""Prepare baseline, cheap evaluation, and experimental deferred-PE candidates.

Source snapshots are isolated; the product checkout is never changed.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

BASELINE_REF = '74570a6933ca843eb79e3148b98e455328e0a62f'
repo = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('output', type=Path)
parser.add_argument('--baseline-ref', default=BASELINE_REF)
args = parser.parse_args()
out = args.output.resolve()
out.mkdir(parents=True)
base = out / 'baseline'
for crate in ['plum', 'ir86']:
    shutil.copytree(repo / crate, base / crate)
shutil.copytree(repo / 'fixtures/frequency-v1', base / 'fixtures/frequency-v1')
shutil.copy(repo / 'Cargo.lock', base / 'Cargo.lock')
(base / 'Cargo.toml').write_text('[workspace]\nresolver="3"\nmembers=["plum","ir86"]\n')
files = ['flags.rs', 'fpu/register_ops.rs', 'simd/exec.rs', 'simd/float.rs', 'simd/mod.rs', 'simd/fxsave.rs']
for name in files:
    path = 'plum/src/' + name
    (base / path).write_bytes(subprocess.check_output(['git', 'show', f'{args.baseline_ref}:{path}'], cwd=repo))
for parent, variant in [('baseline', 'cheap'), ('cheap', 'deferred')]:
    dest = out / variant
    shutil.copytree(out / parent, dest)
    patch = repo / 'scripts/bench/floating-flags-candidates' / (variant + '.patch')
    subprocess.run(['patch', '-d', str(dest), '-p1', '--batch'], input=patch.read_bytes(), check=True)
metadata = dict(baseline_ref=args.baseline_ref, rustc=subprocess.check_output(['rustc','-Vv'],text=True),
                sources={v: {str(p.relative_to(out/v)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for root in ['plum','ir86','docs'] for p in sorted((out/v/root).rglob('*')) if p.is_file()}
                         for v in ['baseline','cheap','deferred']})
(out / 'metadata.json').write_text(json.dumps(metadata, indent=2)+'\n')
print(out)
