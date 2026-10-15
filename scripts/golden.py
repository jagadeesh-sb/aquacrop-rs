"""Run AquaCrop-OSPy on the reference scenarios and save its outputs.

    python scripts/golden.py            # writes tests/golden/<scenario>.json

Run it on Linux (CI does): AquaCrop-OSPy gives last-bit different daily
values on Windows, whose maths library differs from glibc.

Needs aquacrop==3.1.0 (and its pandas/numpy). Each file holds the scenario as
the Rust crate's JSON input, AquaCrop-OSPy's daily tables and final
statistics, and the versions that produced them. `cargo test --test golden`
runs every scenario through the Rust port and compares day by day.

A scenario that AquaCrop-OSPy itself cannot run is written with its error
message, so the Rust test checks that the port fails too.
"""

from __future__ import annotations

import json
import math
import sys
import traceback
from pathlib import Path

import numpy as np
import pandas as pd

import aquacrop
from aquacrop import (
    CO2,
    AquaCropModel,
    Crop,
    FieldMngt,
    GroundWater,
    InitialWaterContent,
    IrrigationManagement,
    Soil,
)
from aquacrop.utils import get_filepath, prepare_weather

OUT = Path(__file__).resolve().parents[1] / "tests" / "golden"


def weather(name: str) -> dict:
    df = prepare_weather(get_filepath(name))
    return {
        "start": df.Date.iloc[0].strftime("%Y-%m-%d"),
        "min_temp": df.MinTemp.astype(float).tolist(),
        "max_temp": df.MaxTemp.astype(float).tolist(),
        "precipitation": df.Precipitation.astype(float).tolist(),
        "reference_et": df.ReferenceET.astype(float).tolist(),
    }


WEATHER: dict[str, dict] = {}


def w(name: str) -> dict:
    if name not in WEATHER:
        WEATHER[name] = weather(name)
    return WEATHER[name]


def scenario(name, sim_start, sim_end, climate, soil, crop, **kw):
    s = {
        "sim_start": sim_start,
        "sim_end": sim_end,
        "weather": w(climate),
        "soil": soil if isinstance(soil, dict) else {"soil_type": soil},
        "crop": crop,
    }
    s.update(kw)
    return name, s


def crop(name, planting, harvest=None, **params):
    c = {"name": name, "planting_date": planting}
    if harvest:
        c["harvest_date"] = harvest
    if params:
        c["params"] = params
    return c


# ---------------------------------------------------------------- building

def py_weather(d: dict) -> pd.DataFrame:
    n = len(d["min_temp"])
    dates = pd.date_range(d["start"], periods=n, freq="D")
    df = pd.DataFrame(
        {
            "MinTemp": d["min_temp"],
            "MaxTemp": d["max_temp"],
            "Precipitation": d["precipitation"],
            "ReferenceET": d["reference_et"],
            "Date": dates,
        }
    )
    if d.get("clip_et0"):
        df["ReferenceET"] = df["ReferenceET"].clip(lower=0.1)
    return df


def py_soil(d: dict) -> Soil:
    kwargs = {k: v for k, v in d.items() if k not in ("soil_type", "layers")}
    s = Soil(d["soil_type"], **kwargs)
    for layer in d.get("layers", []):
        pen = layer.get("penetrability", 100)
        if "sand" in layer:
            s.add_layer_from_texture(layer["thickness"], layer["sand"], layer["clay"], layer["organic_matter"], pen)
        else:
            s.add_layer(layer["thickness"], layer["th_wp"], layer["th_fc"], layer["th_s"], layer["ksat"], pen)
    return s


def py_crop(d: dict) -> Crop:
    return Crop(d["name"], planting_date=d["planting_date"], harvest_date=d.get("harvest_date"), **d.get("params", {}))


def py_irrigation(d: dict | None) -> IrrigationManagement | None:
    if not d:
        return None
    d = dict(d)
    method = d.pop("irrigation_method", 0)
    if "Schedule" in d:
        events = d.pop("Schedule")
        d["Schedule"] = pd.DataFrame(
            {"Date": pd.to_datetime([e["date"] for e in events]), "Depth": [e["depth"] for e in events]}
        )
    return IrrigationManagement(irrigation_method=method, **d)


def py_field(d: dict | None) -> FieldMngt | None:
    return FieldMngt(**d) if d else None


def py_groundwater(d: dict | None) -> GroundWater | None:
    if not d:
        return None
    return GroundWater(
        water_table="Y" if d.get("water_table") else "N",
        method=d.get("method", "Constant"),
        dates=[x.replace("-", "/") for x in d.get("dates", [])],
        values=d.get("values", []),
    )


def py_co2(d: dict | None) -> CO2 | None:
    if not d:
        return None
    kw = {k: v for k, v in d.items() if k in ("ref_concentration", "current_concentration", "constant_conc", "scenario")}
    if d.get("co2_data"):
        kw["co2_data"] = pd.DataFrame(d["co2_data"], columns=["year", "ppm"])
    return CO2(**kw)


def py_iwc(d: dict | None) -> InitialWaterContent:
    if not d:
        return InitialWaterContent()
    return InitialWaterContent(
        wc_type=d.get("wc_type", "Prop"),
        method=d.get("method", "Layer"),
        depth_layer=d.get("depth_layer", [1]),
        value=d.get("value", ["FC"]),
    )


def clean(v):
    """NaN as null; infinities as strings, since JSON has no number for them."""
    v = float(v)
    if math.isnan(v):
        return None
    if math.isinf(v):
        return "inf" if v > 0 else "-inf"
    return v


def table(df: pd.DataFrame) -> dict:
    return {"columns": list(df.columns), "values": [[clean(x) for x in df[c].to_numpy(dtype=float)] for c in df.columns]}


def run(s: dict) -> dict:
    model = AquaCropModel(
        sim_start_time=s["sim_start"].replace("-", "/"),
        sim_end_time=s["sim_end"].replace("-", "/"),
        weather_df=py_weather(s["weather"]),
        soil=py_soil(s["soil"]),
        crop=py_crop(s["crop"]),
        initial_water_content=py_iwc(s.get("initial_water_content")),
        irrigation_management=py_irrigation(s.get("irrigation")),
        field_management=py_field(s.get("field_management")),
        fallow_field_management=py_field(s.get("fallow_field_management")),
        groundwater=py_groundwater(s.get("groundwater")),
        co2_concentration=py_co2(s.get("co2")),
        off_season=s.get("off_season", False),
    )
    model.run_model(till_termination=True)
    fs = model._outputs.final_stats
    final = [
        {
            "season": int(r.iloc[0]),
            "crop_type": str(r.iloc[1]),
            "harvest_date": pd.Timestamp(r.iloc[2]).strftime("%Y-%m-%d"),
            "harvest_step": int(r.iloc[3]),
            "dry_yield": clean(r.iloc[4]),
            "fresh_yield": clean(r.iloc[5]),
            "yield_potential": clean(r.iloc[6]),
            "seasonal_irrigation": clean(r.iloc[7]),
        }
        for _, r in fs.iterrows()
    ]
    return {
        "water_flux": table(model._outputs.water_flux),
        "water_storage": table(model._outputs.water_storage),
        "crop_growth": table(model._outputs.crop_growth),
        "final_stats": final,
    }


# ---------------------------------------------------------------- scenarios

FC = {"wc_type": "Prop", "method": "Layer", "depth_layer": [1], "value": ["FC"]}


def scenarios():
    tunis_wheat = crop("Wheat", "10/01")
    # The AquaCrop-OSPy quick start, and the comparison runs against FAO
    # AquaCrop for Windows (docs/notebooks/05_comparison.ipynb).
    yield scenario("tunis_wheat", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat)
    iwc_depth = {"wc_type": "Num", "method": "Depth", "depth_layer": [0.3, 0.9], "value": [0.3, 0.15]}
    yield scenario("fao_tunis_1", "1979-10-15", "2002-05-30", "tunis_climate.txt", "ac_TunisLocal",
                   crop("WheatGDD", "10/15"), initial_water_content=iwc_depth)
    yield scenario("fao_tunis_1_sandyloam", "1979-01-01", "2002-05-30", "tunis_climate.txt", "SandyLoam",
                   crop("WheatGDD", "10/15"), initial_water_content=iwc_depth)
    yield scenario("fao_tunis_2_long", "1979-01-01", "2002-05-31", "tunis_climate.txt", "SandyLoam",
                   crop("WheatGDD", "10/15", Emergence=289, MaxRooting=1322, Senescence=2835, Maturity=3390,
                        HIstart=2252, Flowering=264, YldForm=1073, PlantPop=3_500_000, CCx=0.9, CDC=0.003888,
                        CGC=0.002734),
                   initial_water_content=iwc_depth)
    yield scenario("fao_tunis_3_30taw", "1979-01-01", "2002-05-31", "tunis_climate.txt", "SandyLoam",
                   crop("WheatGDD", "10/15"),
                   initial_water_content={"wc_type": "Pct", "method": "Layer", "depth_layer": [1], "value": [30]})
    yield scenario("fao_tunis_6_net_irrigation", "1979-08-15", "2001-07-30", "tunis_climate.txt", "SandyLoam",
                   crop("WheatGDD", "12/01", "07/30"),
                   initial_water_content={"wc_type": "Prop", "method": "Layer", "depth_layer": [1], "value": ["WP"]},
                   irrigation={"irrigation_method": 4})
    bund = {"bunds": True, "z_bund": 0.2}
    yield scenario("fao_paddy_hyderabad", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "Paddy",
                   crop("localpaddy", "08/01"),
                   initial_water_content={"wc_type": "Prop", "method": "Layer", "depth_layer": [1, 2], "value": ["FC", "FC"]},
                   field_management=bund, fallow_field_management=bund)
    yield scenario("fao_potato_brussels", "1976-01-01", "2005-12-31", "brussels_climate.txt", "Loam",
                   crop("PotatoLocalGDD", "04/25"))

    # Irrigation methods.
    yield scenario("irr_smt", "1979-10-01", "1986-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   irrigation={"irrigation_method": 1, "SMT": [40, 60, 70, 30]})
    yield scenario("irr_smt_default", "1982-05-01", "1990-10-30", "champion_climate.txt", "Loam",
                   crop("MaizeGDD", "05/01"), irrigation={"irrigation_method": 1})
    yield scenario("irr_interval", "1979-10-01", "1986-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   irrigation={"irrigation_method": 2, "IrrInterval": 7, "AppEff": 80, "WetSurf": 60})
    events = []
    for y in range(1979, 1986):
        for md in ("11-15", "12-20", "02-10", "03-15", "04-10"):
            yr = y if md >= "07" else y + 1
            events.append({"date": f"{yr}-{md}", "depth": 25.0})
    yield scenario("irr_schedule", "1979-10-01", "1986-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   irrigation={"irrigation_method": 3, "Schedule": events, "MaxIrr": 30})
    yield scenario("irr_net", "1979-10-01", "1986-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   irrigation={"irrigation_method": 4, "NetIrrSMT": 70})
    yield scenario("irr_depth", "1979-10-01", "1986-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   irrigation={"irrigation_method": 5, "depth": 3.5, "MaxIrrSeason": 300})

    # Water table.
    yield scenario("gw_constant", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   groundwater={"water_table": True, "dates": ["1979-10-01"], "values": [2.66]})
    yield scenario("gw_shallow_clay", "1979-10-01", "1985-05-30", "tunis_climate.txt", "ClayLoam", tunis_wheat,
                   groundwater={"water_table": True, "dates": ["1979-10-01"], "values": [1.2]})
    yield scenario("gw_variable", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SiltLoam", tunis_wheat,
                   groundwater={"water_table": True, "method": "Variable", "dates": ["1979-10-01", "1984-04-30"],
                                "values": [0.25, 2.0]})
    yield scenario("gw_steps", "1979-10-01", "1985-05-30", "tunis_climate.txt", "Loam", tunis_wheat,
                   groundwater={"water_table": True, "method": "Constant",
                                "dates": ["1979-10-01", "1981-06-01", "1983-01-15"], "values": [1.5, 0.8, 3.0]})

    # Field management and the off-season.
    yield scenario("field_mulch_cn", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   field_management={"mulches": True, "mulch_pct": 80, "f_mulch": 0.6, "curve_number_adj": True,
                                     "curve_number_adj_pct": -10},
                   fallow_field_management={"mulches": True, "mulch_pct": 20})
    yield scenario("field_runoff_inhibited", "1979-10-01", "1985-05-30", "tunis_climate.txt", "ClayLoam", tunis_wheat,
                   field_management={"sr_inhb": True})
    yield scenario("off_season", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   off_season=True,
                   fallow_field_management={"mulches": True, "mulch_pct": 20, "bunds": True, "bund_water": 30,
                                            "z_bund": 1})

    # CO2.
    yield scenario("co2_constant_550", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   co2={"constant_conc": True, "current_concentration": 550})
    yield scenario("co2_series", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam", tunis_wheat,
                   co2={"co2_data": [[1979, 400], [1981, 480], [1983, 620], [1985, 900]]})

    # Soils and initial water.
    custom = {"soil_type": "custom", "cn": 72, "rew": 9,
              "layers": [{"thickness": 0.4, "sand": 25, "clay": 45, "organic_matter": 1.5},
                         {"thickness": 1.6, "sand": 40, "clay": 30, "organic_matter": 0.5, "penetrability": 80}]}
    yield scenario("soil_custom_texture", "1979-10-01", "1985-05-30", "tunis_climate.txt", custom, tunis_wheat,
                   initial_water_content={"wc_type": "Pct", "method": "Depth", "depth_layer": [0.2, 1.0], "value": [70, 40]})
    custom2 = {"soil_type": "custom", "dz": [0.05] * 4 + [0.1] * 6 + [0.25] * 4, "adj_rew": 0, "calc_cn": 1,
               "evap_z_surf": 0.06, "z_top": 0.15,
               "layers": [{"thickness": 0.3, "th_wp": 0.12, "th_fc": 0.28, "th_s": 0.45, "ksat": 300},
                          {"thickness": 1.5, "th_wp": 0.2, "th_fc": 0.36, "th_s": 0.48, "ksat": 60, "penetrability": 60}]}
    yield scenario("soil_custom_hydraulic", "1979-10-01", "1985-05-30", "tunis_climate.txt", custom2, tunis_wheat,
                   initial_water_content={"wc_type": "Prop", "method": "Layer", "depth_layer": [1, 2], "value": ["SAT", "WP"]})
    yield scenario("soil_tile_drainage", "2000-01-01", "2005-12-31", "hyderabad_climate.txt",
                   {"soil_type": "Paddy", "tile_drainage": True, "drain_depth": 0.6, "drain_coeff": 0.3},
                   crop("PaddyRiceGDD", "07/15"),
                   field_management={"bunds": True, "z_bund": 0.15, "bund_water": 50})

    # Crop features.
    yield scenario("switch_gdd", "1979-10-01", "1986-05-30", "tunis_climate.txt", "SandyLoam",
                   crop("Wheat", "10/01", SwitchGDD=1))
    yield scenario("adaptive_planting", "2000-01-01", "2006-12-31", "hyderabad_climate.txt", "ClayLoam",
                   crop("MaizeGDD", "06/01", PlantTrigger=True, PlantTempThr=24, PlantTWindow=5, PlantRainThr=40,
                        PlantRainWindow=10, PlantMaxDelay=45))
    yield scenario("crop_overrides", "1979-10-01", "1985-05-30", "tunis_climate.txt", "SandyLoam",
                   crop("Wheat", "10/01", "06/15", CCx=0.85, WP=30, HI0=0.42, Zmax=1.2, PolHeatStress=0,
                        TrColdStress=0, dHI_pre=5, LagAer=4, Aer=6))

    # Indian kharif and rabi crops on the Hyderabad weather.
    yield scenario("in_cotton_clay", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "Clay", crop("Cotton", "06/20"))
    yield scenario("in_cotton_gdd", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "ClayLoam", crop("CottonGDD", "06/20"))
    yield scenario("in_maize_kharif", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "SandyClayLoam",
                   crop("Maize", "06/25"))
    yield scenario("in_sorghum_rabi", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "Clay",
                   crop("Sorghum", "10/15"), irrigation={"irrigation_method": 1, "SMT": [60, 60, 50, 30]})
    yield scenario("in_paddy_gdd", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "Paddy",
                   crop("PaddyRiceGDD", "07/10"), field_management=bund,
                   initial_water_content={"wc_type": "Prop", "method": "Layer", "depth_layer": [1, 2], "value": ["SAT", "FC"]})
    yield scenario("in_sugarcane", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "ClayLoam",
                   crop("SugarCane", "02/01"), irrigation={"irrigation_method": 2, "IrrInterval": 10})
    yield scenario("in_tomato", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "SandyLoam",
                   crop("Tomato", "11/01"), irrigation={"irrigation_method": 1, "SMT": [70, 70, 70, 70]})
    yield scenario("in_soybean", "2000-01-01", "2010-12-31", "hyderabad_climate.txt", "SiltClayLoam",
                   crop("Soybean", "06/25"))

    # Every built-in crop once, on weather that suits it.
    from aquacrop.entities.crops.crop_params import crop_params
    plant = {"Wheat": "10/15", "Barley": "10/15", "Maize": "05/01", "Potato": "04/25", "SugarBeet": "04/01",
             "Sunflower": "04/15", "Tomato": "05/01", "Cotton": "05/01", "DryBean": "05/01", "Soybean": "05/01",
             "Sorghum": "05/01", "PaddyRice": "05/01", "Quinoa": "04/15", "Tef": "05/01", "SugarCane": "03/01",
             "Alfalfa": "04/01", "Cassava": "04/01", "Default": "05/01", "localpaddy": "07/15"}
    for name in crop_params:
        base = next((k for k in plant if name.startswith(k) or name.startswith("Hyd") and k == "Wheat"), "Default")
        climate = "hyderabad_climate.txt" if name in ("localpaddy",) or name.startswith("Hyd") else "champion_climate.txt"
        start, end = ("2000-01-01", "2006-12-31") if climate.startswith("hyderabad") else ("1982-01-01", "1990-12-31")
        yield scenario(f"preset_{name}", start, end, climate, "Loam", crop(name, plant[base]))


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    only = set(sys.argv[1:])
    meta = {"aquacrop": aquacrop.__version__ if hasattr(aquacrop, "__version__") else "3.1.0",
            "pandas": pd.__version__, "numpy": np.__version__, "python": sys.version.split()[0]}
    ok = failed = 0
    for name, s in scenarios():
        if only and name not in only:
            continue
        doc = {"name": name, "input": s, "upstream": meta}
        try:
            doc["expected"] = run(json.loads(json.dumps(s)))
            ok += 1
            seasons = doc["expected"]["final_stats"]
            print(f"{name}: {len(seasons)} seasons, mean dry yield "
                  f"{np.mean([x['dry_yield'] or 0 for x in seasons]) if seasons else float('nan'):.3f}")
        except Exception as e:  # AquaCrop-OSPy itself rejects the scenario
            doc["error"] = f"{type(e).__name__}: {e}"
            failed += 1
            print(f"{name}: AquaCrop-OSPy fails: {doc['error'].splitlines()[0][:160]}")
            if "-v" in only:
                traceback.print_exc()
        (OUT / f"{name}.json").write_text(json.dumps(doc, separators=(",", ":")))
    print(f"{ok} scenarios written, {failed} that AquaCrop-OSPy cannot run")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
