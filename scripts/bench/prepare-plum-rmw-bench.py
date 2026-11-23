"""Prepare identical harnesses for current HEAD versus the working tree.

This reads Git without changing it; snapshots and release builds stay in output.
"""
import shutil
import subprocess
import sys
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
output = Path(sys.argv[1]).resolve()
assert not output.exists(), f'Choose a fresh output directory: {output}'
tracked = subprocess.check_output(['git', 'ls-files', '-z', 'plum', 'ir86'], cwd=repo).decode().split('\0')
for stage in ['before', 'after']:
    root = output / stage
    for crate in ['plum', 'ir86']:
        shutil.copytree(repo / crate, root / crate, ignore=shutil.ignore_patterns('target'))
    if stage == 'before':
        for name in filter(None, tracked):
            (root / name).write_bytes(subprocess.check_output(['git', 'show', f'HEAD:{name}'], cwd=repo))
    # The new examples/test references are shared harness source, even for HEAD.
    shutil.copytree(repo / 'plum/examples', root / 'plum/examples', dirs_exist_ok=True)
    for name in ['rmw.rs', 'rmw_lowering.rs']:
        shutil.copy(repo / 'plum/tests' / name, root / 'plum/tests' / name)
    (root / 'Cargo.toml').write_text('[workspace]\nresolver="3"\nmembers=["plum","ir86"]\n[profile.release]\nopt-level="s"\nlto=true\ncodegen-units=1\npanic="abort"\n')
    shutil.copy(repo / 'Cargo.lock', root / 'Cargo.lock')
    # A temporary directory otherwise uses the global default, which can differ
    # from the repository's compiler and change dispatcher code generation.
    shutil.copy(repo / 'rust-toolchain.toml', root / 'rust-toolchain.toml')
    print(root)
print('Baseline:', subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip())
