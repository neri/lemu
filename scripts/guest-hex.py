"""Encode fixed binary guests as text, or decode them into a build directory."""
import argparse
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode', choices=['encode', 'decode'])
parser.add_argument('source', type=Path)
parser.add_argument('output', type=Path)
args = parser.parse_args()
if args.source.resolve() == args.output.resolve():
    parser.error('source and output must be different files')
args.output.parent.mkdir(parents=True, exist_ok=True)
if args.mode == 'encode':
    data = args.source.read_bytes()
    args.output.write_text(''.join(data[i:i + 32].hex(' ') + '\n'
                                   for i in range(0, len(data), 32)))
else:
    args.output.write_bytes(bytes.fromhex(args.source.read_text()))
