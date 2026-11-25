"""Runs every timing inside one container and writes /results/bench.json."""
import glob, json, os, statistics, subprocess, time

G = "/src/tests/golden"
PICK = ["tunis_wheat", "fao_paddy_hyderabad", "fao_tunis_1", "fao_potato_brussels"]
ENGINES = {
    "python": ["python", "/bench/py_bench.py"],
    "rust": ["/work/target/release/acbench"],
    "wasm": ["node", "/bench/wasm_bench.mjs", "/src/pkg"],
}
out = {"cpu": open("/proc/cpuinfo").read().split("model name")[1].split("\n")[0].strip(": "), "nproc": os.cpu_count()}


def call(engine, reps, files):
    t = time.perf_counter()
    p = subprocess.run(ENGINES[engine] + [str(reps)] + files, capture_output=True, text=True, check=True)
    wall = (time.perf_counter() - t) * 1000
    lines = [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]
    imp = next((l["import_ms"] for l in lines if "import_ms" in l), 0.0)
    return wall, imp, [l for l in lines if "file" in l]


def save():
    json.dump(out, open("/results/bench.json", "w"), indent=1)


# One-time cost of Rust: a clean release build (LTO, one codegen unit), crates already fetched.
subprocess.run(["cargo", "fetch", "--quiet"], cwd="/work", check=True)
t = time.perf_counter()
subprocess.run(["cargo", "build", "--release", "--quiet"], cwd="/work", check=True)
out["rust_build_s"] = time.perf_counter() - t
# First Python import in a fresh container (imports numpy, pandas, aquacrop).
out["python_first_import_ms"] = call("python", 1, [])[1]
save()
print("build done", flush=True)

# Time to first result: a fresh process per run.
out["cold"] = {}
for name in PICK:
    f = f"{G}/{name}.json"
    out["cold"][name] = {}
    for eng in ENGINES:
        trials = 3 if eng == "python" else 7
        rows = [call(eng, 1, [f]) for _ in range(trials)]
        out["cold"][name][eng] = {
            "wall_ms": statistics.median(r[0] for r in rows),
            "import_ms": statistics.median(r[1] for r in rows),
            "run_ms": statistics.median(r[2][0]["ms"][0] for r in rows),
            "yield_sum": rows[0][2][0]["yield_sum"],
        }
        print(name, eng, round(out["cold"][name][eng]["wall_ms"]), flush=True)
    save()

# Repeat runs in one process (after the first).
out["warm"] = {}
for eng, reps in [("python", 3), ("rust", 30), ("wasm", 30)]:
    _, _, rows = call(eng, reps, [f"{G}/{n}.json" for n in PICK])
    out["warm"][eng] = {os.path.basename(r["file"])[:-5]: {"first_ms": r["ms"][0], "median_ms": statistics.median(r["ms"][1:])} for r in rows}
    save()
print("warm done", flush=True)

# Every scenario, one run each, in one process per engine.
files = sorted(glob.glob(f"{G}/*.json"))
meta = {}
for f in files:
    d = json.load(open(f))
    s = d["input"]
    days = (time.mktime(time.strptime(s["sim_end"], "%Y-%m-%d")) - time.mktime(time.strptime(s["sim_start"], "%Y-%m-%d"))) / 86400 + 1
    meta[os.path.basename(f)[:-5]] = {"years": days / 365.25, "error": bool(d.get("error"))}
out["all"] = {"meta": meta}
for eng in ENGINES:
    wall, imp, rows = call(eng, 1, files)
    out["all"][eng] = {"wall_ms": wall, "import_ms": imp,
                       "per": {os.path.basename(r["file"])[:-5]: {"ms": r["ms"][0], "yield_sum": r["yield_sum"]} for r in rows}}
    save()
    print("all", eng, round(wall), flush=True)
