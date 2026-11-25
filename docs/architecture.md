# Architecture and modules

aquacrop-rs is a line-by-line port of AquaCrop-OSPy 3.1.0. The code is laid
out so that each Rust file can be read next to the Python file it came from.
This page explains how the crate is divided, what each part does, and the
order in which the daily processes run.

## Design rules

- **Python names are kept.** Each of the 23 files in `src/solution/` has the
  name of an AquaCrop-OSPy module in `aquacrop/solution/`. The eight small
  helper modules sit in the file that calls them: `cc_development`,
  `cc_required_time`, `adjust_CCx` and `update_CCx_CDC` in `canopy_cover.rs`;
  the three `HIadj_*` modules in `harvest_index.rs`; and
  `evap_layer_water_content` in `soil_evaporation.rs`. Function names and
  variable names are kept, including Python's mixed case (`NewCond`, `th_fc`,
  `CCx`). This is why the crate allows `non_snake_case` and several Clippy
  lints.
- **Same arithmetic, same order.** Statements keep the Python shape
  (`x = x + y`, explicit index loops), so a floating-point sum is done in the
  same order as in Python. This is what makes 51 of the 65 reference scenarios
  identical in every bit.
- **Python's library behaviour is copied where results depend on it.**
  `src/round.rs` reproduces the two roundings AquaCrop-OSPy uses (Python's
  `round` and NumPy's `round`), which give different answers at decimal ties.
  `src/numeric.rs` holds the NumPy functions the model needs (`cumsum`,
  `interp` and similar) with NumPy's edge-case behaviour.
- **No crash where Python crashes.** Where AquaCrop-OSPy raises an exception or
  loops forever, the port returns an `Error` or takes a defined branch. Every
  such place is marked `Deviation:` in the source. See
  [differences.md](differences.md).
- **One dependency for the model.** The model uses only `serde` and
  `serde_json`. The WebAssembly build adds `wasm-bindgen`.

## Layout

| Path | Lines | What it holds | AquaCrop-OSPy source |
|---|---:|---|---|
| `src/lib.rs` | 66 | Crate root, public API, `UPSTREAM_VERSION` | |
| `src/config.rs` | 296 | `ModelInput` and every input struct, with defaults | `entities/*.py`, `AquaCropModel(...)` arguments |
| `src/model.rs` | 72 | `Model` (step by step) and `run` (whole simulation) | `core.py` |
| `src/init.rs` | 980 | Everything done before day one: clock, weather checks, crop calendar, CO2, irrigation and groundwater tables, initial water content | `initialize/*.py` |
| `src/timestep.rs` | 496 | One simulated day, season changes, harvest and yield | `timestep/*.py` |
| `src/state.rs` | 299 | Clock, parameters and the daily state (`InitCond`) | `entities/clockStruct.py`, `paramStruct.py`, `initParamVariables.py` |
| `src/crop.rs` | 428 | Crop parameters, the 37 built-in crops, overrides | `entities/crop.py`, `data/crop_params` |
| `src/soil.rs` | 512 | Soil types, layers, compartments, Saxton and Rawls (2006) | `entities/soil.py` |
| `src/output.rs` | 85 | `Results`: daily tables and season statistics | `entities/output.py` |
| `src/date.rs` | 160 | Calendar dates without a date library | |
| `src/round.rs` | 94 | Python and NumPy rounding | |
| `src/numeric.rs` | 146 | NumPy helpers | |
| `src/error.rs` | 19 | `Error` type | |
| `src/wasm.rs` | 42 | JavaScript bindings (feature `wasm`) | |
| `src/solution/` | 23 files | The daily processes, below | `solution/*.py` |
| `data/` | | Crop parameter table and CO2 series, unchanged from AquaCrop-OSPy | `data/` |

The crate has 6,813 lines of Rust in `src/`, comments included.

## The daily processes

Every simulated day, `src/timestep.rs` calls the process modules in this
order. The order is the same as in AquaCrop-OSPy, and it matters: each step
reads the soil water and crop state the previous step left.

| # | Module | What it computes |
|---:|---|---|
| 1 | `growing_degree_day` | Growing degree days from the day's minimum and maximum temperature |
| 2 | `check_groundwater_table` | Water table depth for the day and whether it is inside the soil profile |
| 3 | `root_development` | Root depth |
| 4 | `pre_irrigation` | Irrigation before sowing, for net irrigation |
| 5 | `drainage` | Water moving down between soil compartments |
| 6 | `tile_drainage` | Water removed by subsurface drains, when present |
| 7 | `rainfall_partition` | Rain split into runoff and infiltration (curve number) |
| 8 | `irrigation` | Irrigation depth for the day, by the chosen method |
| 9 | `infiltration` | Water entering the soil, bunds and surface storage |
| 10 | `capillary_rise` | Water rising from a shallow water table |
| 11 | `germination` | Whether the crop has germinated |
| 12 | `growth_stage` | Current growth stage |
| 13 | `canopy_cover` | Green canopy cover, with and without water stress |
| 14 | `soil_evaporation` | Evaporation from the soil surface (two stages) |
| 15 | `transpiration` | Crop transpiration and water taken from each compartment |
| 16 | `groundwater_inflow` | Water entering from the water table |
| 17 | `hi_ref_current_day` | Reference harvest index for the day |
| 18 | `biomass_accumulation` | Biomass from transpiration and water productivity |
| 19 | `harvest_index` | Harvest index adjusted for stress |
| 20 | `root_zone_water` | Water in the root zone, for the daily output |

Three helpers are called from inside these modules: `water_stress` (stress
coefficients for canopy growth, stomata, senescence and pollination),
`temperature_stress` (heat and cold stress on biomass and pollination) and
`aeration_stress` (stress from a waterlogged root zone). `root_zone_water` is
also called by several modules during the day.

After the processes, `timestep.rs` computes the day's yield, writes one row to
each daily table, and checks for harvest, the end of a season and the start of
the next.

## Public API

```rust
// Whole simulation in one call.
pub fn run(input: &ModelInput) -> Result<Results, Error>;

// Step by step, for callers that want to stop early or inspect the state.
impl Model {
    pub fn new(input: &ModelInput) -> Result<Model, Error>;
    pub fn step(&mut self) -> Result<(), Error>;      // one day
    pub fn is_finished(&self) -> bool;
    pub fn run_to_end(&mut self) -> Result<(), Error>;
    pub fn results(&self) -> Results;
}

pub fn crop::crop_names() -> Vec<String>;
pub fn crop::crop_preset(name: &str) -> Option<&'static Map<String, Value>>;
pub const UPSTREAM_VERSION: &str = "3.1.0";
```

The WebAssembly build exposes `run`, `crop_names`, `crop_preset`,
`soil_types` and `version`. See [webassembly.md](webassembly.md).
