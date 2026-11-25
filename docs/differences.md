# Where the port differs from AquaCrop-OSPy

The port follows AquaCrop-OSPy's arithmetic, including two of its less
obvious behaviours:

- rainfall partitioning shortens the depth of the compartment that crosses
  the curve-number depth, and the shorter depth stays for the rest of the run;
- net irrigation on the first day of a season changes the water content that
  the following seasons start from.

Both are kept because changing them would change the results.

The port differs only where AquaCrop-OSPy cannot finish: it raises an
exception, refers to a variable that was never set, or loops forever. In
those places the port returns an `Error` or takes a defined branch. Each one
is marked `Deviation:` in the source, so `grep -rn "Deviation:" src` lists
them all.

## Inputs the port rejects with an error

| Place | AquaCrop-OSPy | aquacrop-rs |
|---|---|---|
| `src/init.rs:98` | Loops forever when the soil profile is shallower than the maximum rooting depth plus 0.1 m and no compartment can grow | Returns an input error |
| `src/init.rs:637` | A water table date outside the simulation period is appended by pandas as an extra row | Returns an input error |

## Cases where the port takes a defined branch

| Place | AquaCrop-OSPy | aquacrop-rs |
|---|---|---|
| `src/timestep.rs:359` | Fails when the next planting date falls outside the simulation | Ends the simulation |
| `src/solution/canopy_cover.rs:25` | Raises for a canopy mode other than growth or decline | Treats it as decline |
| `src/solution/canopy_cover.rs:118` | A calendar type other than 1 or 2 leaves variables unset | Reads it as 2 (growing degree days) |
| `src/solution/growth_stage.rs:12` | Same | Same |
| `src/solution/root_development.rs:39` | Same | Same |
| `src/solution/growing_degree_day.rs:24` | A GDD method other than 1, 2 or 3 leaves `gdd` unset | Reads it as method 3 |
| `src/solution/check_groundwater_table.rs:24` | A negative water table depth leaves a flag unset | The water table is outside the soil |
| `src/solution/harvest_index.rs:53` | `HIt < 0` leaves `FracFlow` unset | 0 (the caller only passes positive values) |
| `src/solution/harvest_index.rs:208` | A crop type other than 1, 2 or 3 leaves the adjusted index unset | Keeps the unadjusted index |
| `src/solution/infiltration.rs:64` | Bunds of zero height leave two variables unset | The field is treated as having no bunds, as rainfall partitioning already does |
| `src/solution/irrigation.rs:89` | An unknown irrigation method leaves `Irr` unset | No irrigation |
| `src/solution/rainfall_partition.rs:47` | Indexes past the last compartment when every compartment ends above the curve-number depth | Stops at the last compartment |
| `src/solution/soil_evaporation.rs:14`, `:194`, `:261` | Indexes past the last compartment when the evaporation layer reaches the bottom of the profile | Stops at the last compartment |
| `src/solution/temperature_stress.rs:12` | A stress flag other than 0 or 1 leaves the coefficient unset | No stress |
| `src/solution/transpiration.rs:77` | A cold stress flag other than 0 or 1 leaves `KsCold` unset | No stress |
| `src/solution/transpiration.rs:197` | Refers to `p_up_sto` before it is set | Uses the unadjusted threshold |

## Scenarios AquaCrop-OSPy 3.1.0 cannot run

Nine of the 74 reference scenarios fail inside AquaCrop-OSPy 3.1.0 (with
numpy 2.5.4 and pandas 3.0.6):

- Eight fail on AquaCrop's own input checks: seven crops "take longer than 1
  year to mature" under the test weather, and one has "not enough growing
  degree days in simulation". The port fails on all eight with the same
  message.
- `preset_AlfalfaGDD` fails in AquaCrop-OSPy with `ValueError: assignment
  destination is read-only`, an incompatibility with pandas 3. The port runs
  it. Its results have nothing to be compared with.

## Not simulated

As in AquaCrop-OSPy 3.1.0: salinity, soil fertility stress, weed management
and perennial herbaceous crops.

## Last-bit differences

AquaCrop-OSPy computes `exp`, `log` and `pow` with the C library of the
machine it runs on. The native port does the same through Rust's standard
library, so on Linux it matches AquaCrop-OSPy on Linux. AquaCrop-OSPy itself
gives slightly different daily values on Windows and on Linux for this
reason, and the reference files are made on Linux. The WebAssembly build
uses Rust's own maths library, which can differ in the last bit; see
[validation.md](validation.md).
