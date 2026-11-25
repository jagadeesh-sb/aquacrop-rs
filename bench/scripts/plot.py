"""Charts for the AquaCrop-OSPy vs aquacrop-rs timing: plot.py <bench.json> <out_dir>"""
import json
import os
import statistics
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import font_manager
from matplotlib.ticker import FixedLocator, NullLocator

# Poppins when AQ_FONT_DIR holds its .ttf files, otherwise matplotlib's default font.
_fonts = list(Path(os.environ.get("AQ_FONT_DIR", ".")).glob("Poppins-*.ttf"))
for f in _fonts:
    font_manager.fontManager.addfont(str(f))
# AQ_PLAIN=1: no title, subtitle or footnote in the image (for documents with their own captions).
PLAIN = os.environ.get("AQ_PLAIN") == "1"
SAVE = {"bbox_inches": "tight", "pad_inches": 0.15} if PLAIN else {}
plt.rcParams.update({"font.family": "Arial" if PLAIN else ("Poppins" if _fonts else "DejaVu Sans"), "font.size": 10.5})

SURFACE, INK, INK2, MUTED, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#8a8984", "#e6e5e1"
ENG = {  # fixed categorical order: slot 1, 2, 3
    "python": ("Python (AquaCrop-OSPy 3.1.0)", "#2a78d6"),
    "rust": ("Rust, native (aquacrop-rs)", "#eb6834"),
    "wasm": ("Rust in the browser (WebAssembly)", "#1baf7a"),
}
SHORT = {"python": "Python", "rust": "Rust", "wasm": "WASM"}
NAMES = {
    "tunis_wheat": "Wheat, Tunis",
    "fao_paddy_hyderabad": "Rice, Hyderabad",
    "fao_tunis_1": "Wheat, Tunis (long)",
    "fao_potato_brussels": "Potato, Brussels",
}

d = json.load(open(sys.argv[1]))
d["cpu"] = d["cpu"].split(":")[-1].strip()
out = Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
win = json.load(open(sys.argv[3])) if len(sys.argv) > 3 else None
meta = d["all"]["meta"]
PICK = list(d["cold"])


def fmt(ms):
    if ms < 10:
        return f"{ms:.1f} ms"
    if ms < 1000:
        return f"{ms:.0f} ms"
    s = ms / 1000
    if s < 60:
        return f"{s:.1f} s"
    return f"{int(s // 60)} min {s % 60:02.0f} s"


def label(name):
    return f"{NAMES[name]}\n{meta[name]['years']:.0f} years simulated"


def style(ax, grid_axis="x"):
    ax.set_facecolor(SURFACE)
    for s in ("top", "right", "left"):
        ax.spines[s].set_visible(False)
    ax.spines["bottom"].set_color(GRID)
    ax.tick_params(colors=INK2, length=0)
    ax.grid(axis=grid_axis, color=GRID, linewidth=0.8)
    ax.set_axisbelow(True)


def titles(fig, title, sub):
    if PLAIN:
        return
    fig.text(0.02, 0.965, title, fontsize=15, fontweight="semibold", color=INK, va="top")
    fig.text(0.02, 0.905, sub, fontsize=10, color=INK2, va="top")


def legend(fig, keys, y=0.835):
    x = 0.02
    for k in keys:
        fig.patches.append(plt.Rectangle((x, y - 0.006), 0.012, 0.022, transform=fig.transFigure,
                                         color=ENG[k][1], figure=fig))
        fig.text(x + 0.018, y + 0.005, ENG[k][0], fontsize=9.5, color=INK2, va="center")
        x += 0.018 + 0.0072 * len(ENG[k][0]) + 0.03


def foot(fig, text):
    if PLAIN:
        return
    import textwrap
    fig.text(0.02, 0.015, textwrap.fill(text, 175), fontsize=8.5, color=MUTED, va="bottom")


def log_ticks(ax, lo, hi, axis="x"):
    vals = [v for v in (1, 10, 100, 1000, 10_000, 60_000, 600_000) if lo <= v <= hi]
    loc = FixedLocator(vals)
    lab = [fmt(v).replace(".0 ", " ").replace("10 s", "10 s") for v in vals]
    if axis == "x":
        ax.xaxis.set_major_locator(loc); ax.xaxis.set_minor_locator(NullLocator()); ax.set_xticklabels(lab)
    else:
        ax.yaxis.set_major_locator(loc); ax.yaxis.set_minor_locator(NullLocator()); ax.set_yticklabels(lab)


def grouped_hbar(ax, rows, values, keys, logx, labeller):
    h = 0.24
    for i, name in enumerate(rows):
        for j, k in enumerate(keys):
            y = i + (j - (len(keys) - 1) / 2) * (h + 0.04)
            v = values[name][k]
            ax.barh(y, v, height=h, color=ENG[k][1], left=0.5 if logx else 0)
            ax.text(v * (1.12 if logx else 1) + (0 if logx else max(max(values[n].values()) for n in rows) * 0.01),
                    y, labeller(name, k, v), va="center", fontsize=9, color=INK)
    ax.set_yticks(range(len(rows)))
    ax.set_yticklabels([label(n) for n in rows], color=INK, fontsize=9.5)
    ax.invert_yaxis()


FOOT = (f"Same scenario files, same machine ({d['cpu']}), all three in one Linux Docker container. "
        "Times include building the model from its inputs.")

# 1. Time to first result from a fresh start ---------------------------------------
cold = {n: {k: d["cold"][n][k]["wall_ms"] for k in ENG} for n in PICK}
fig, ax = plt.subplots(figsize=(11, 6.2), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.2, right=0.93, top=0.76, bottom=0.15)
style(ax)
ax.set_xscale("log")
grouped_hbar(ax, PICK, cold, list(ENG), True, lambda n, k, v: fmt(v))
lo = min(min(r.values()) for r in cold.values()); hi = max(max(r.values()) for r in cold.values())
ax.set_xlim(lo / 2, hi * 4)
log_ticks(ax, lo / 2, hi * 4)
r_cold = statistics.median(cold[n]["python"] / cold[n]["rust"] for n in PICK)
titles(fig, "Time to first result from a fresh start",
       "A new process for each run: start-up, then one full simulation. Log scale.")
legend(fig, list(ENG))
foot(fig, FOOT + " Median of 3 (Python) or 7 (Rust) fresh starts; each includes process start and reading the scenario file (1 to 3.5 MB).")
fig.savefig(out / "1_time_to_first_result.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

# 2. How many times faster, repeat runs ---------------------------------------------
warm = {n: {k: d["warm"][k][n]["median_ms"] for k in ENG} for n in PICK}
speed = {n: {k: warm[n]["python"] / warm[n][k] for k in ("rust", "wasm")} for n in PICK}
fig, ax = plt.subplots(figsize=(11, 6.2), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.2, right=0.93, top=0.76, bottom=0.15)
style(ax)
grouped_hbar(ax, PICK, speed, ["rust", "wasm"], False,
             lambda n, k, v: f"{v:,.0f}x   ({fmt(warm[n][k])} vs {fmt(warm[n]['python'])})")
ax.set_xlim(0, max(max(r.values()) for r in speed.values()) * 1.45)
ax.xaxis.set_major_formatter(matplotlib.ticker.FuncFormatter(lambda v, _: f"{v:,.0f}x"))
titles(fig, "Speed-up over AquaCrop-OSPy on repeat runs",
       "Median of repeat runs in a running program. AquaCrop-OSPy = 1x.")
legend(fig, ["rust", "wasm"])
foot(fig, FOOT + " Python: median of 2 repeats; Rust: median of 29.")
fig.savefig(out / "2_times_faster.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

# 3. Run time against length of simulation, every scenario -------------------------
ok = [n for n, m in meta.items() if not m["error"]]
fig, ax = plt.subplots(figsize=(11, 6.2), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.1, right=0.85, top=0.76, bottom=0.14)
style(ax, "y")
ax.set_yscale("log")
for k in ENG:
    xs = [meta[n]["years"] for n in ok]
    ys = [d["all"][k]["per"][n]["ms"] for n in ok]
    ax.scatter(xs, ys, s=34, color=ENG[k][1], edgecolor=SURFACE, linewidth=1.2, zorder=3)
    per_year = statistics.median(y / x for x, y in zip(xs, ys))
    ax.text(max(xs) * 1.03, {"python": 4000, "wasm": 120, "rust": 16}[k],
            f"{SHORT[k]}\n{fmt(per_year)} per year simulated", fontsize=9, color=INK, va="center")
ys_all = [d["all"][k]["per"][n]["ms"] for k in ENG for n in ok]
ax.set_ylim(min(ys_all) / 2, max(ys_all) * 2)
log_ticks(ax, min(ys_all) / 2, max(ys_all) * 2, "y")
ax.set_xlabel("Years simulated", color=INK2)
ax.set_xlim(0, max(meta[n]["years"] for n in ok) * 1.25)
titles(fig, f"Run time against years simulated, {len(ok)} scenarios",
       "One run per scenario. Log scale.")
legend(fig, list(ENG))
foot(fig, FOOT)
fig.savefig(out / "3_time_vs_years.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

# 4. All scenarios end to end --------------------------------------------------------
tot = {k: d["all"][k]["wall_ms"] for k in ENG}
fig, ax = plt.subplots(figsize=(11, 4.6), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.2, right=0.93, top=0.66, bottom=0.17)
style(ax)
keys = list(ENG)
for i, k in enumerate(keys):
    ax.barh(i, tot[k], height=0.42, color=ENG[k][1])
    ax.text(tot[k] + max(tot.values()) * 0.01, i, fmt(tot[k]), va="center", fontsize=11,
            fontweight="semibold", color=INK)
ax.set_yticks(range(len(keys)))
ax.set_yticklabels([SHORT[k] if k != "wasm" else "Rust in browser\n(WebAssembly)" for k in keys], color=INK)
ax.invert_yaxis()
ax.set_xlim(0, max(tot.values()) * 1.15)
ax.xaxis.set_major_locator(FixedLocator([0, 30_000, 60_000, 90_000, 120_000]))
ax.set_xticklabels(["0", "30 s", "1 min", "1 min 30 s", "2 min"])
n_all = len(meta)
titles(fig, f"Time to run all {n_all} reference scenarios",
       "One program per engine, from start-up to the last result. Nine scenarios stop early on an input check in every engine.")
foot(fig, FOOT)
fig.savefig(out / "4_all_scenarios.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

summary = {
    "cold_ms": cold, "warm_ms": warm, "speedup_warm": speed, "total_ms": tot,
    "rust_build_s": d["rust_build_s"], "python_first_import_ms": d["python_first_import_ms"],
    "cold_detail": d["cold"],
}
json.dump(summary, open(out / "summary.json", "w"), indent=1)
print(json.dumps(summary, indent=1))
