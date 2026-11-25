# Output reference

`run` returns a `Results` value. As JSON it has six fields.

| Field | Meaning |
|---|---|
| `start` | Date of the first row in the daily tables |
| `water_flux` | Daily water balance table |
| `water_storage` | Daily water content of each soil compartment |
| `crop_growth` | Daily crop table |
| `final_stats` | One entry per harvested season |
| `compartments` | Compartment thicknesses (m) the model used, after any deepening of the profile to fit the roots |

Each daily table is `{"columns": [...], "values": [[...], ...]}` with one
array per column and one value per day. Column names are AquaCrop-OSPy's, so
code written for its `water_flux`, `water_storage` and `crop_growth` data
frames reads the same names.

Days the model skips (between a harvest and the next planting, unless
`off_season` is set) and the last day of the period stay zero, as in
AquaCrop-OSPy. JSON has no value for NaN or infinity, so both are written as
`null`. In the daily tables `null` marks a value AquaCrop-OSPy holds as None,
such as the water table depth when there is no water table. In
`final_stats`, crops with no dry-matter content give an infinite fresh yield
in AquaCrop-OSPy, which is written as `null`.

## `water_flux`

| Column | Unit | Meaning |
|---|---|---|
| `time_step_counter` | | Day number from the start |
| `season_counter` | | Season number |
| `dap` | days | Days after planting |
| `Wr` | mm | Water in the root zone |
| `z_gw` | m | Water table depth |
| `surface_storage` | mm | Water stored on the surface by bunds |
| `IrrDay` | mm | Irrigation |
| `Infl` | mm | Infiltration |
| `Runoff` | mm | Surface runoff |
| `DeepPerc` | mm | Deep percolation below the profile |
| `TileDrain` | mm | Water removed by tile drains |
| `CR` | mm | Capillary rise |
| `GwIn` | mm | Inflow from the water table |
| `Es` | mm | Soil evaporation |
| `EsPot` | mm | Potential soil evaporation |
| `Tr` | mm | Transpiration |
| `TrPot` | mm | Potential transpiration |

## `water_storage`

`time_step_counter`, `growing_season`, `dap`, then `th1`, `th2`, ... one
column per soil compartment, each the water content in m3/m3.

## `crop_growth`

| Column | Unit | Meaning |
|---|---|---|
| `time_step_counter`, `season_counter`, `dap` | | As above |
| `gdd` | degC day | Growing degree days for the day |
| `gdd_cum` | degC day | Growing degree days since planting |
| `z_root` | m | Root depth |
| `canopy_cover` | fraction | Green canopy cover |
| `canopy_cover_ns` | fraction | Canopy cover without water stress |
| `biomass` | g/m2 | Above-ground biomass |
| `biomass_ns` | g/m2 | Biomass without water stress |
| `harvest_index` | fraction | Reference harvest index |
| `harvest_index_adj` | fraction | Harvest index adjusted for stress |
| `DryYield` | t/ha | Dry yield so far |
| `FreshYield` | t/ha | Fresh yield so far |
| `YieldPot` | t/ha | Yield without water stress so far |

## `final_stats`

| Field | Unit | Meaning |
|---|---|---|
| `season` | | Season number, from 0 |
| `crop_type` | | Crop name |
| `harvest_date` | date | The day after the last simulated day of the season |
| `harvest_step` | | Day number of the harvest |
| `dry_yield` | t/ha | Dry yield |
| `fresh_yield` | t/ha | Fresh yield |
| `yield_potential` | t/ha | Yield without water stress |
| `seasonal_irrigation` | mm | Irrigation applied in the season |

In Rust, `results.water_flux.column("Tr")` returns one column as a slice.
