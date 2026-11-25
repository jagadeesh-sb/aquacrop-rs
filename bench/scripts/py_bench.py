"""Times AquaCrop-OSPy on golden scenario files: py_bench.py <reps> <file.json>..."""
import time
T0 = time.perf_counter()
import json, sys
import os
sys.path.insert(0, os.environ.get("AQ_SCRIPTS", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts")))
import golden  # imports numpy, pandas, aquacrop
from aquacrop import AquaCropModel
T_IMPORT = (time.perf_counter() - T0) * 1000
print(json.dumps({"import_ms": T_IMPORT}), flush=True)


def run(s):
    m = AquaCropModel(
        sim_start_time=s["sim_start"].replace("-", "/"), sim_end_time=s["sim_end"].replace("-", "/"),
        weather_df=golden.py_weather(s["weather"]), soil=golden.py_soil(s["soil"]), crop=golden.py_crop(s["crop"]),
        initial_water_content=golden.py_iwc(s.get("initial_water_content")),
        irrigation_management=golden.py_irrigation(s.get("irrigation")),
        field_management=golden.py_field(s.get("field_management")),
        fallow_field_management=golden.py_field(s.get("fallow_field_management")),
        groundwater=golden.py_groundwater(s.get("groundwater")), co2_concentration=golden.py_co2(s.get("co2")),
        off_season=s.get("off_season", False),
    )
    m.run_model(till_termination=True)
    return float(m._outputs.final_stats.iloc[:, 4].sum())


reps = int(sys.argv[1])
for path in sys.argv[2:]:
    doc = json.load(open(path))
    times, y = [], float("nan")
    for _ in range(max(reps, 1)):
        t = time.perf_counter()
        try:
            y = run(doc["input"])
        except Exception:
            pass
        times.append((time.perf_counter() - t) * 1000)
    print(json.dumps({"file": path, "ms": times, "yield_sum": y}), flush=True)
