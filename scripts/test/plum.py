"""Run PLUM's three suites and reject missing, ignored, or filtered contracts."""
import argparse
import re
import shlex
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cargo', default='cargo')
parser.add_argument('--suite', choices=['common', 'harvard', 'flat'])
parser.add_argument('--matrix', action='store_true')
args = parser.parse_args()

configs = [
    ('default', [], True, True, False),
    ('minimal', ['--no-default-features'], False, False, False),
    ('x87', ['--no-default-features', '--features', 'x87'], True, False, False),
    ('all', ['--all-features'], True, True, True),
] if args.matrix else [('default', [], True, True, False)]

def run_contracts(name, suite, features, target, expected):
    command = [*shlex.split(args.cargo), 'test', '-p', 'plum',
               *features, *target]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    counts = re.findall(
        r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; '
        r'(\d+) measured; (\d+) filtered out;', result.stdout)
    wanted = [(str(expected), '0', '0', '0', '0')]
    if result.returncode or counts != wanted:
        print(' '.join(command), flush=True)
        print(result.stdout + result.stderr, flush=True)
        raise SystemExit(f'{name}/{suite}: expected {wanted}, got {counts}')
    print(f'{name}/{suite} {target[0]}: {expected} passed', flush=True)


for name, features, x87, sse, profile in configs:
    flag_elision = '--all-features' in features
    targets = {
        'common': [(['--test', 'common'], 9),
                   (['--lib', 'common_'], 4 + 6 * x87 + 9 * sse + profile)],
        'harvard': [(['--test', 'harvard'], 26 + sse + 3 * profile)],
        'flat': [(['--test', 'flat'], 20 + sse + 4 * profile + flag_elision)],
    }
    if flag_elision:
        targets['harvard'].append((['--test', 'flag_elision'], 10))
    for suite in [args.suite] if args.suite else targets:
        for target, expected in targets[suite]:
            run_contracts(name, suite, features, target, expected)
    if args.suite is None:
        run_contracts(name, 'linux32', features, ['--test', 'linux32'], 2)
    if args.matrix and args.suite is None:
        # Explicit --test targets omit examples; check every feature configuration too.
        subprocess.run([*shlex.split(args.cargo), 'check', '-p', 'plum',
                        *features, '--examples'], cwd=ROOT, check=True)
