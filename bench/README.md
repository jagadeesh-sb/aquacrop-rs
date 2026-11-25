# Benchmarks and comparison charts

The scripts here measure AquaCrop-OSPy 3.1.0, aquacrop-rs (native) and
aquacrop-rs (WebAssembly) on the reference scenarios, compare their outputs,
and draw the charts in `docs/images/`. Results from 18 November 2026 are in
`results/`; the write-up is in [docs/performance.md](../docs/performance.md)
and [docs/validation.md](../docs/validation.md).

Everything runs in one Docker container, so the three engines share the same
machine and operating system.

## Steps

From the repository root:

```sh
# 1. Reference scenarios (needs Python with aquacrop==3.1.0, numpy==2.5.4, pandas==3.0.6)
python scripts/golden.py

# 2. WebAssembly package in pkg/ (see docs/webassembly.md)

# 3. Image with Python 3.12, AquaCrop-OSPy, Rust and Node.js
docker build -t acbench bench/scripts

# 4. Timings: writes bench/results/bench.json (about 15 minutes, mostly Python)
mkdir -p bench/work bench/out
cp -r bench/scripts/acbench bench/work/
docker run --rm -e AQ_SCRIPTS=/src/scripts \
  -v "$PWD:/src:ro" -v "$PWD/bench/scripts:/bench:ro" \
  -v "$PWD/bench/work/acbench:/work" -v "$PWD/bench/out:/results" \
  acbench python /bench/driver.py

# 5. Full outputs of the port, for the comparison
cp -r bench/scripts/acdump bench/work/
docker run --rm \
  -v "$PWD:/src:ro" -v "$PWD/bench/scripts:/bench:ro" \
  -v "$PWD/bench/work/acdump:/work" -v "$PWD/bench/out:/results" \
  acbench bash /bench/dump.sh

# 6. Charts (needs matplotlib)
python bench/scripts/plot.py bench/out/bench.json docs/images
python bench/scripts/compare.py tests/golden bench/out/rust bench/out/wasm docs/images
```

The two small crates in `scripts/acbench` and `scripts/acdump` depend on
aquacrop-rs by the path `/src`, which is where step 4 and 5 mount the
repository.

Set `AQ_FONT_DIR` to a folder holding the Poppins `.ttf` files to draw the
charts in Poppins; otherwise matplotlib's default font is used.

## Test coverage

```sh
cargo install cargo-llvm-cov
cargo llvm-cov --release --summary-only --json --output-path bench/out/coverage.json
python bench/scripts/coverage_chart.py bench/out/coverage.json docs/images/8_test_coverage.png
```

## What each script does

| Script | Purpose |
|---|---|
| `driver.py` | Runs all timings inside the container and writes `bench.json` |
| `py_bench.py` | Times AquaCrop-OSPy on scenario files |
| `acbench/` | Times the native port on scenario files |
| `wasm_bench.mjs` | Times the WebAssembly build in Node.js |
| `acdump/`, `wasm_dump.mjs`, `dump.sh` | Write the port's full outputs for every scenario |
| `plot.py` | Timing charts 1 to 4 |
| `compare.py` | Agreement charts 5 to 7, and `agreement.json` |
| `coverage_chart.py` | Coverage chart 8 |
| `Dockerfile` | The benchmark image |
