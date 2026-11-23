"""Build identical ordinary algorithms for x87/SSE2, at O2/Os (no fast-math)."""
import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parents[2]
source = root / 'plum/examples/support/bench/floating-flags'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, help='Directory for a separate compiler/input snapshot')
args = parser.parse_args()
output = args.output.resolve() if args.output else source / 'guest'
output.mkdir(parents=True, exist_ok=True)
flags = ['-m32', '-march=i686', '-ffreestanding', '-fno-pic', '-fno-pie',
         '-fno-stack-protector', '-fno-asynchronous-unwind-tables', '-fno-builtin',
         '-fno-math-errno', '-ffp-contract=off']
rows = []
with tempfile.TemporaryDirectory(prefix='plum-guests-') as directory:
    build = Path(directory)
    for backend, extra in [('x87', ['-mfpmath=387', '-mno-sse', '-mno-sse2']),
                           ('sse2', ['-mfpmath=sse', '-msse2'])]:
        for case, name in enumerate(['audio', 'transform', 'statistics', 'finance']):
            for opt in ['O2', 'Os']:
                stem = f'{backend}-{name}-{opt}'
                obj = build / f'{stem}.o'
                elf = obj.with_suffix('.elf')
                start = build / 'start.o'
                compile_flags = [*flags, *extra, f'-{opt}', f'-DCASE={case}']
                subprocess.run(['gcc', *compile_flags, '-c', str(source/'workloads.c'), '-o', str(obj)], check=True)
                subprocess.run(['gcc', '-m32', '-c', str(source/'start.S'), '-o', str(start)], check=True)
                subprocess.run(['ld', '-m', 'elf_i386', '-T', str(source/'link.ld'), str(start), str(obj), '-o', str(elf)], check=True)
                binary = build / f'{stem}.bin'
                subprocess.run(['objcopy', '-O', 'binary', str(elf), str(binary)], check=True)
                data = binary.read_bytes()
                (output/f'{stem}.bin.hex').write_text(''.join(data[i:i+32].hex(' ')+'\n' for i in range(0,len(data),32)))
                (output/f'{stem}.disasm').write_text(subprocess.check_output(['objdump', '-d', str(elf)], text=True))
                rows.append(dict(id=len(rows), name=stem, backend=backend, family=name,
                                 output_address=0x20000, output_size=(24576 if case == 1 else 16384 if case == 0 else 8192),
                                 compiler_flags=compile_flags, sha256=hashlib.sha256(binary.read_bytes()).hexdigest()))
(output/'manifest.json').write_text(json.dumps(dict(compiler=subprocess.check_output(['gcc','--version'],text=True).splitlines()[0], cases=rows), indent=2)+'\n')
print(output)
