set -e
mkdir -p /results/rust /results/wasm
cd /work && cargo build --release --quiet
/work/target/release/acdump /results/rust /src/tests/golden/*.json
node /bench/wasm_dump.mjs /src/pkg /results/wasm /src/tests/golden/*.json
ls /results/rust | wc -l; ls /results/wasm | wc -l
