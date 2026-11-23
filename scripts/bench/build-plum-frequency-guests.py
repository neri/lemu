"""Build fixed compiler-generated, freestanding i386 inputs and metadata."""
import argparse
import shutil
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
archive = repo / 'fixtures/frequency-v1'
parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,help='Fresh directory for another compiler/input snapshot')
args=parser.parse_args()
root=args.output.resolve() if args.output else archive
if args.output:
    if root.exists():raise SystemExit(f'Choose a fresh output directory: {root}')
    (root/'guest').mkdir(parents=True)
    for name in ['workloads.c','start.S','link.ld']:shutil.copy(archive/'guest'/name,root/'guest'/name)
elif (archive/'metadata.json').exists():
    raise SystemExit('v1 is frozen: use the saved guest hex files, or --output NEW_DIRECTORY to generate a separate snapshot')
source = root / 'guest'
flags = ['-m32', '-march=i386', '-ffreestanding', '-fno-pic', '-fno-pie',
         '-fno-stack-protector', '-fno-asynchronous-unwind-tables', '-fno-unroll-loops',
         '-mno-sse', '-mno-mmx', '-msoft-float']
rows = []
with tempfile.TemporaryDirectory(prefix='plum-guests-') as directory:
    build = Path(directory)
    for family, name in enumerate(['integer', 'byte-word', 'updates', 'calls', 'copy-compare', 'updates-immediate', 'updates-register']):
        for opt in ['O2', 'Os']:
            stem = f'{name}-{opt}'
            obj = build / f'{stem}.o'
            subprocess.run(['gcc', *flags, f'-{opt}', f'-DCASE={family}', '-c', str(source/'workloads.c'), '-o', str(obj)], check=True)
            start = build / 'start.o'
            subprocess.run(['gcc', '-m32', '-c', str(source/'start.S'), '-o', str(start)], check=True)
            elf = build / f'{stem}.elf'
            subprocess.run(['ld', '-m', 'elf_i386', '-T', str(source/'link.ld'), str(start), str(obj), '-o', str(elf)], check=True)
            binary = build / f'{stem}.bin'
            subprocess.run(['objcopy', '-O', 'binary', '-j', '.text', str(elf), str(binary)], check=True)
            data = binary.read_bytes()
            (source/f'{stem}.bin.hex').write_text(''.join(data[i:i+32].hex(' ')+'\n' for i in range(0,len(data),32)))
            (source/f'{stem}.disasm').write_text(subprocess.check_output(['objdump', '-d', str(elf)], text=True))
            rows.append(dict(id=len(rows), name=stem, family='updates' if name.startswith('updates') else name, family_weight=.2, case_weight=1/6 if name.startswith('updates') else .5,
                             iterations=256, code_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), compiler_flags=flags+[f'-{opt}',f'-DCASE={family}']))
manifest = dict(version=1, compiler=subprocess.check_output(['gcc','--version'],text=True).splitlines()[0],
                code_base=1048576, data_size=65536, stack_pointer=65536,
                input='data[i] = ((i*37 + (i>>3)*11) ^ (i>>5)) & 255 for i=0..65535',
                interval='cached execute through INT 40, excluding construction and initial execute; 256 complete guest invocations per sample',
                profile='release opt-level=s lto=true codegen-units=1 panic=abort; counters disabled',
                warmups=3, samples=9, batches=5, cases=rows)
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
