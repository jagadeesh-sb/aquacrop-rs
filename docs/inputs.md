# Input reference

A simulation takes one `ModelInput`, written as JSON. Field names and
defaults follow AquaCrop-OSPy's `AquaCropModel(...)` and its entity classes,
so a model set up in Python can be moved across with few changes. Unknown
field names are rejected, which catches typing mistakes.

Dates are `"YYYY-MM-DD"`. Planting and harvest dates are `"MM/DD"`, as in
AquaCrop-OSPy.

## Top level

| Field | Required | Meaning |
|---|---|---|
| `sim_start` | yes | First simulated day |
| `sim_end` | yes | Last day of the simulation period |
| `weather` | yes | Daily weather, see below |
| `soil` | yes | Soil, see below |
| `crop` | yes | Crop and planting date, see below |
| `initial_water_content` | no | Soil water on the first day. Default: field capacity |
| `irrigation` | no | Irrigation method and settings. Default: rainfed |
| `field_management` | no | Mulches, bunds, runoff settings in the growing season |
| `fallow_field_management` | no | The same, between seasons |
| `groundwater` | no | Water table. Default: none |
| `co2` | no | CO2 concentration. Default: Mauna Loa observations |
| `off_season` | no | `true` to simulate the days between harvest and the next planting. Default `false` |

## `weather`

One value per day, starting on `start`. The four series must have the same
length and must cover the simulation period. Missing or non-numeric values
are rejected.

| Field | Unit | Meaning |
|---|---|---|
| `start` | date | Date of the first value |
| `min_temp` | degC | Daily minimum air temperature |
| `max_temp` | degC | Daily maximum air temperature |
| `precipitation` | mm | Daily precipitation |
| `reference_et` | mm | Daily reference evapotranspiration (ET0) |
| `clip_et0` | | `true` raises ET0 below 0.1 mm to 0.1 mm, as AquaCrop-OSPy's `prepare_weather` does. Default `false` |

## `soil`

| Field | Default | Meaning |
|---|---|---|
| `soil_type` | | A built-in soil (below) or `"custom"` |
| `layers` | `[]` | Layers added below the built-in soil's own, or the whole profile for `"custom"` |
| `dz` | twelve of 0.1 | Compartment thicknesses (m) |
| `rew` | 9 | Readily evaporable water (mm) |
| `cn` | 61 | Curve number |
| `calc_cn` | 0 | 1 to compute the curve number from saturated conductivity |
| `adj_rew`, `adj_cn` | 1 | Adjust REW and CN to the soil (1) or not (0) |
| `z_res` | none | Depth of a restrictive layer (m) |
| `evap_z_surf`, `evap_z_min`, `evap_z_max` | 0.04, 0.15, 0.30 | Evaporation layer depths (m) |
| `kex`, `f_evap`, `f_wrel_exp`, `fwcc` | 1.1, 4, 0.4, 50 | Soil evaporation coefficients |
| `z_cn`, `z_germ`, `z_top` | 0.3, 0.3, 0.1 | Depths (m) for curve number, germination and the top soil |
| `fshape_cr` | 16 | Capillary rise shape factor |
| `tile_drainage` | `false` | Subsurface drains present |
| `drain_depth`, `drain_target`, `drain_coeff` | none, none, 1 | Drain settings |

Built-in soils: `Clay`, `ClayLoam`, `Default`, `Loam`, `LoamySand`, `Sand`,
`SandyClay`, `SandyClayLoam`, `SandyLoam`, `Silt`, `SiltClayLoam`,
`SiltLoam`, `SiltClay`, `Paddy` and `ac_TunisLocal`.

Each entry in `layers` has a `thickness` (m), an optional `penetrability`
(% of roots that can enter, default 100), and one of two property sets:

- `th_wp`, `th_fc`, `th_s` (water content at wilting point, field capacity and
  saturation, m3/m3) and `ksat` (saturated conductivity, mm/day); or
- `sand`, `clay` and `organic_matter` (%), from which the hydraulic properties
  are computed by Saxton and Rawls (2006), as in AquaCrop-OSPy.

The last layer continues to the bottom of the profile. When the crop roots
deeper than the profile, the profile is deepened to fit them.

## `crop`

| Field | Meaning |
|---|---|
| `name` | A built-in crop (below) or `"custom"` |
| `planting_date` | `"MM/DD"` |
| `harvest_date` | `"MM/DD"`, the latest harvest date. Optional; derived from maturity when absent |
| `params` | Parameter overrides by AquaCrop-OSPy name, for example `{"CCx": 0.9, "WP": 33.7, "Maturity": 1200}` |

The 37 built-in crops are AquaCrop-OSPy's: `AlfalfaGDD`, `Barley`,
`BarleyGDD`, `Cassava`, `Cotton`, `CottonGDD`, `Default`, `DryBean`,
`DryBeanGDD`, `HydWheatGDD`, `localpaddy`, `Maize`, `MaizeChampionGDD`,
`MaizeGDD`, `PaddyRice`, `PaddyRiceGDD`, `Potato`, `PotatoGDD`,
`PotatoLocalGDD`, `Quinoa`, `Sorghum`, `SorghumGDD`, `Soybean`,
`SoybeanGDD`, `SugarBeet`, `SugarBeetGDD`, `SugarBeetGDD_UK`, `SugarCane`,
`Sunflower`, `SunflowerGDD`, `Tef`, `Tomato`, `TomatoGDD`, `Wheat`,
`WheatGDD`, `WheatGDD_1dec` and `WheatLongGDD`. Names ending in `GDD` are
calibrated in growing degree days; the others in calendar days.
`crop::crop_preset(name)` returns every parameter of a built-in crop.

A `"custom"` crop must give every parameter in `params`. The error message
lists any that are missing.

## `initial_water_content`

| Field | Default | Meaning |
|---|---|---|
| `wc_type` | `"Prop"` | `"Prop"` (values `"WP"`, `"FC"`, `"SAT"`), `"Num"` (m3/m3) or `"Pct"` (% of total available water) |
| `method` | `"Layer"` | `"Layer"` (one value per soil layer) or `"Depth"` (values at depths, interpolated) |
| `depth_layer` | `[1]` | Layer numbers or depths (m) |
| `value` | `["FC"]` | One value per entry in `depth_layer` |

## `irrigation`

| Field | Default | Meaning |
|---|---|---|
| `irrigation_method` | 0 | 0 rainfed, 1 soil-moisture targets, 2 fixed interval, 3 schedule, 4 net irrigation, 5 constant depth every day |
| `SMT` | 100 each | Method 1: soil-moisture targets for the four growth stages (% of total available water) |
| `IrrInterval` | 3 | Method 2: days between irrigations |
| `Schedule` | `[]` | Method 3: list of `{"date": "YYYY-MM-DD", "depth": mm}` |
| `NetIrrSMT` | 80 | Method 4: target root-zone water (% of total available water) |
| `depth` | 0 | Method 5: depth applied every day (mm) |
| `MaxIrr` | 25 | Largest single irrigation (mm) |
| `MaxIrrSeason` | 10000 | Largest seasonal total (mm) |
| `AppEff` | 100 | Application efficiency (%) |
| `WetSurf` | 100 | Share of the surface wetted (%) |

## `field_management` and `fallow_field_management`

| Field | Default | Meaning |
|---|---|---|
| `mulches` | `false` | Mulch present |
| `mulch_pct` | 50 | Share of the surface covered (%) |
| `f_mulch` | 0.5 | Reduction of soil evaporation by the mulch |
| `bunds` | `false` | Bunds present |
| `z_bund` | 0 | Bund height (m) |
| `bund_water` | 0 | Water held by the bunds on the first day (mm) |
| `curve_number_adj` | `false` | Adjust the curve number |
| `curve_number_adj_pct` | 0 | Adjustment (%) |
| `sr_inhb` | `false` | Surface runoff stopped |

## `groundwater`

| Field | Default | Meaning |
|---|---|---|
| `water_table` | `false` | A water table is present |
| `method` | `"Constant"` | `"Constant"` (steps at each date) or `"Variable"` (interpolated between dates) |
| `dates` | `[]` | Dates of the depths, inside the simulation period |
| `values` | `[]` | Water table depths (m) |

## `co2`

| Field | Default | Meaning |
|---|---|---|
| `scenario` | none | A built-in series: `"ssp119"`, `"ssp126"`, `"ssp245"`, `"ssp370"`, `"ssp434"`, `"ssp460"`, `"ssp534os"`, `"ssp585"`. Mauna Loa observations when absent |
| `co2_data` | none | Your own series as `[[year, ppm], ...]`, used instead of the built-in ones |
| `constant_conc` | `false` | Hold the concentration at `current_concentration` |
| `current_concentration` | 0 | ppm, with `constant_conc` |
| `ref_concentration` | 369.41 | Reference concentration (ppm) |

## A complete example

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

The weather arrays are cut short here; a real input has one value per day.
Every file in `tests/golden/` (made by `python scripts/golden.py`) holds a
full input under its `input` key.
