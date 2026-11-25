# aquacrop-rs

A Rust port of [AquaCrop-OSPy](https://github.com/aquacropos/aquacrop) 3.1.0,
the open-source Python implementation of the FAO AquaCrop crop-water
productivity model. It simulates canopy cover, rooting, soil water, soil
evaporation, transpiration, biomass and yield day by day, for one field and
any number of seasons. It builds as an ordinary Rust library and as a
626 KB WebAssembly module that runs in the browser.

The port follows the Python code module by module and keeps its variable
names, so each formula can be read side by side with the original. It is not
an official implementation of FAO AquaCrop, and neither is AquaCrop-OSPy.

## At a glance

Measured on 18 November 2026 against AquaCrop-OSPy 3.1.0 on the same
machine ([performance](docs/performance.md), [validation](docs/validation.md)):

| | AquaCrop-OSPy | aquacrop-rs | aquacrop-rs in the browser |
|---|---:|---:|---:|
| One 23-year run, repeated | 5.3 s | 29 ms | 58 ms |
| All 74 reference scenarios, start to finish | 1 min 57 s | 1.8 s | 3.1 s |
| Scenarios identical in every bit (of 65) | | 51 | 0 |
| Largest season yield difference (623 seasons) | | 0 | 0.00076% |

The native build runs the reference scenarios 123 to 184 times faster than
AquaCrop-OSPy once both are running, and 92.9% of its lines are run by the
tests.

![Times faster than AquaCrop-OSPy](docs/images/2_times_faster.png)

## Documentation

[White paper (PDF)](docs/aquacrop-rs-white-paper.pdf) ·
[Inputs](docs/inputs.md) ·
[Outputs](docs/outputs.md) ·
[WebAssembly](docs/webassembly.md) ·
[Architecture and modules](docs/architecture.md) ·
[Differences from AquaCrop-OSPy](docs/differences.md) ·
[Validation](docs/validation.md) ·
[Performance](docs/performance.md) ·
[Benchmarks](bench/README.md) ·
[Contributing](CONTRIBUTING.md)

## Agreement with AquaCrop-OSPy

`scripts/golden.py` runs AquaCrop-OSPy 3.1.0 (numpy 2.5.4, pandas 3.0.6) on
74 scenarios and saves every daily output: the AquaCrop-OSPy quick start, the
comparison runs against FAO AquaCrop for Windows that ship with AquaCrop-OSPy
(wheat at Tunis, rice at Hyderabad, potato at Brussels), every irrigation
method, constant and varying water tables, mulches, bunds, the off-season,
CO2 series, custom and texture-based soils, tile drainage, adaptive planting,
crop parameter overrides and every built-in crop. Nine of the scenarios fail
inside AquaCrop-OSPy; the port fails on the eight that fail on AquaCrop's own
checks and runs the ninth, which fails on a pandas 3 incompatibility.

`cargo test --test golden` compares every column of every day:

- On Linux, with the Python references also generated on Linux, all 74
  scenarios agree to 1e-9 and 51 of them are bit-identical on every value of
  every day. In the rest the first difference is in the last bit of a
  soil-evaporation value, after the exponential function, typically hundreds
  of days into the run.
- The WebAssembly build uses Rust's own maths library, whose `exp`, `ln` and
  `pow` can differ from glibc's in the last bit. `node scripts/wasm-check.mjs`
  finds 70 scenarios agreeing day by day to 1e-9. In the other four, a
  threshold (drainage, or the two-decimal rounding of root-zone water) tips a
  day apart; their season yields and irrigation still agree to within a
  relative 1.1e-5.

AquaCrop-OSPy itself gives slightly different daily values on Windows and on
Linux for the same reason, so references are generated on Linux.

## Use

```toml
[dependencies]
aquacrop = { git = "https://github.com/jagadeesh-sb/aquacrop-rs" }
```

```rust
let input: aquacrop::ModelInput = serde_json::from_str(&std::fs::read_to_string("run.json")?)?;
let results = aquacrop::run(&input)?;
for s in &results.final_stats {
    println!("season {} harvested {}: {:.2} t/ha", s.season, s.harvest_date, s.dry_yield);
}
```

The input mirrors the arguments of `AquaCropModel` and uses AquaCrop-OSPy's
names and defaults:

```json
{
  "sim_start": "2000-01-01",
  "sim_end": "2010-12-31",
  "weather": {
    "start": "2000-01-01",
    "min_temp": [11.3, 11.3],
    "max_temp": [27.3, 26.7],
    "precipitation": [0.0, 0.0],
    "reference_et": [3.6, 3.7]
  },
  "soil": { "soil_type": "Paddy" },
  "crop": { "name": "localpaddy", "planting_date": "08/01" },
  "initial_water_content": { "wc_type": "Prop", "method": "Layer", "depth_layer": [1, 2], "value": ["FC", "FC"] },
  "field_management": { "bunds": true, "z_bund": 0.2 },
  "irrigation": { "irrigation_method": 1, "SMT": [60, 60, 50, 30] }
}
```

Crop parameters can be overridden by name under `crop.params` (for example
`"CCx"`, `"WP"`, `"Maturity"`). A custom soil takes `layers`, each with either
`th_wp`, `th_fc`, `th_s` and `ksat`, or `sand`, `clay` and `organic_matter`
(Saxton and Rawls 2006). The results hold the daily `water_flux`,
`water_storage` and `crop_growth` tables, one array per column, and
`final_stats` with one entry per harvested season.

### In the browser

```sh
cargo build --profile wasm --target wasm32-unknown-unknown --features wasm
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/wasm/aquacrop.wasm
```

```js
import init, { run, crop_names } from './pkg/aquacrop.js'
await init()
const results = JSON.parse(run(JSON.stringify(input)))
```

`run` throws a JavaScript error when the input cannot be simulated.

## Where the port differs

The port follows AquaCrop-OSPy's arithmetic, including two of its less
obvious behaviours: rainfall partitioning shortens the depth of the
compartment that crosses the curve-number depth for the rest of a run, and
net irrigation on the first day of a season changes the water content the
following seasons start from. Where AquaCrop-OSPy raises an exception or
loops forever, the port returns an error or takes the sensible branch
instead. Each such place is marked `Deviation:` in the source. Among them:

- a soil too shallow for the roots, which AquaCrop-OSPy extends without end;
- `ETadj` other than 1, where AquaCrop-OSPy refers to an undefined variable;
- an evaporation or rainfall loop that would index past the bottom of the
  profile;
- a next planting date outside the simulation period, which ends the run;
- a water-table date outside the simulation period, which is rejected.

Salinity, fertility stress and weed management are not simulated, as in
AquaCrop-OSPy 3.1.0.

## Testing

```sh
pip install aquacrop==3.1.0 numpy==2.5.4 pandas==3.0.6
python scripts/golden.py        # writes tests/golden/, about 90 MB
cargo test
node scripts/wasm-check.mjs     # after building pkg/
```

## Licence and credit

Licensed under Apache-2.0, like AquaCrop-OSPy. The model, its parameter sets and its data are
the work of the AquaCrop-OSPy authors (Foster, Kelly and contributors); see
NOTICE. Please cite their paper when you use this port:

Kelly, T. D. and Foster, T. (2021). AquaCrop-OSPy: Bridging the gap between
research and practice in crop-water modeling. Agricultural Water Management,
254, 106976. https://doi.org/10.1016/j.agwat.2021.106976
