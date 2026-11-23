"""Collect native paired cumulative and warmed samples; pin the driver using taskset."""
import argparse
import csv
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("baseline")
parser.add_argument("candidate")
parser.add_argument("output")
parser.add_argument("--batches", type=int, default=10)
args = parser.parse_args()
with open(args.output, "w") as stream:
    writer = csv.writer(stream, lineterminator='\n')
    writer.writerow(["batch", "case", "n", "a_ns", "b_ns", "eax_sum", "data_hash"])
    for batch in range(args.batches):
        for offset in range(16):
            case = (offset + 5 * batch) % 16
            results = {}
            pair = [args.baseline, args.candidate]
            for binary in pair[::1 if (batch + case) % 2 == 0 else -1]:
                output = subprocess.check_output([binary, str(case), "series"], text=True)
                results[binary] = list(csv.reader(output.splitlines()))
            for a, b in zip(*(results[path] for path in pair), strict=True):
                assert a[0] == b[0] and a[2:] == b[2:], (case, a, b)
                writer.writerow([batch, case, a[0], a[1], b[1], *a[2:]])
        stream.flush()
