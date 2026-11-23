# Validation and measurement tools

Run from the repository root:

| Command | Purpose | Requirements |
| --- | --- | --- |
| `make test` | Workspace defaults plus PLUM common/Harvard/Flat feature matrix | Rust; GCC/binutils with i386 support for Linux32 |
| `make test-wasm` | Small RMW/load/flat/SIMD contracts and Linux32 correctness in Node | Rust Wasm target, Node, GCC/binutils |
| `make test-tools` | Benchmark scoring and uncertainty analysis | Python 3 |

`make test-common`, `make test-harvard` and `make test-flat`
run one suite with expected-count checks. `make test-matrix` runs all four
feature configurations. The old BCD `make test-oracle` entry has been retired.

`test/` contains correctness runners and validators. `bench/` contains timing,
profiling, corpus construction, and research comparisons. Timing is kept separate
from correctness, and profile counters must be disabled for timed builds.
`test/check-sharing-profiles.py` and `bench/verify-plum-frequency.py` need existing
measurement artifacts; they are not standalone test suites.

Use `CARGO_NET_OFFLINE=true make ...` after dependencies are cached. Install the
Wasm target with `rustup target add wasm32-unknown-unknown` if needed. `make` and
`mise run` expose the same suite names. See [TESTING.md](../docs/TESTING.md) for scope,
case ownership, expected omissions, and the migration record.
