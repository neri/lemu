"""Build each feature configuration immediately before running its Wasm probes."""
import argparse
import json
import os
from pathlib import Path
import subprocess

repo = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cargo', default='cargo')
parser.add_argument('--node', default='node')
args = parser.parse_args()


def run(command):
    print('+', *map(str, command), flush=True)
    subprocess.run(command, cwd=repo, check=True)


# Respect CARGO_TARGET_DIR and workspace target-dir configuration.
metadata = json.loads(subprocess.check_output(
    [args.cargo, 'metadata', '--format-version', '1', '--no-deps'], cwd=repo))
artifacts = Path(metadata['target_directory']) / 'wasm32-unknown-unknown/release/examples'


def probe(example, features, runner, *extra):
    command = [args.cargo, 'build', '-p', 'plum', '--release',
               '--target', 'wasm32-unknown-unknown', '--no-default-features',
               '--example', example]
    if features:
        command += ['--features', features]
    run(command)
    run([args.node, repo / 'scripts/test' / runner,
         artifacts / (example + '.wasm'), *extra])


probe('leave_probe', '', 'test-plum-leave-wasm.mjs')
probe('leave_probe', 'sse', 'test-plum-leave-wasm.mjs')
probe('rmw_dispatch_bench', '', 'test-plum-rmw-wasm.mjs')
probe('load_extend_bench', '', 'load-wasm.mjs', 'extend')
probe('narrow_load_bench', '', 'load-wasm.mjs', 'narrow')
probe('flat_memory_probe', '', 'test-plum-flat-wasm.mjs')
probe('flat_memory_probe', 'x87', 'test-plum-flat-wasm.mjs')
# SSE: NaN results and MXCSR flags must not depend on the host.
native = subprocess.check_output(
    [args.cargo, 'run', '-q', '-p', 'plum', '--release', '--no-default-features',
     '--features', 'sse', '--example', 'simd_probe'], cwd=repo, text=True).strip()
probe('simd_probe', 'sse', 'test-plum-simd-wasm.mjs', native)
# The same with simd128: the lane operations compile to Wasm SIMD.
simd128 = Path(metadata['target_directory']) / 'wasm-simd128'
subprocess.run([args.cargo, 'build', '-p', 'plum', '--release', '--target', 'wasm32-unknown-unknown',
                '--no-default-features', '--features', 'sse', '--example', 'simd_probe',
                '--target-dir', simd128], cwd=repo, check=True,
               env={**os.environ, 'RUSTFLAGS': '-C target-feature=+simd128'})
run([args.node, repo / 'scripts/test/test-plum-simd-wasm.mjs',
     simd128 / 'wasm32-unknown-unknown/release/examples/simd_probe.wasm', native])
run(['sh', 'plum/examples/linux32/build.sh'])
probe('linux32', '', 'linux32-wasm.mjs', repo / 'target/linux32/jit-calc')
fixtures = repo / 'plum/examples/support/simd'
probe('linux32', 'sse', 'linux32-sse-wasm.mjs', fixtures / 'sse-calc.elf.hex', fixtures / 'sse-calc.out')
