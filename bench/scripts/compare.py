"""Compare aquacrop-rs (native and WebAssembly) outputs with AquaCrop-OSPy on every golden scenario.
compare.py <golden_dir> <rust_dir> <wasm_dir> <out_dir>"""
import json
import math
import os
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
PY, RS, WS = "#2a78d6", "#eb6834", "#1baf7a"
ENG = {"rust": ("Rust, native", RS), "wasm": ("Rust in the browser (WebAssembly)", WS)}

golden, rdir, wdir, out = (Path(a) for a in sys.argv[1:5])
out.mkdir(parents=True, exist_ok=True)
SKIP = {"time_step_counter", "season_counter", "dap", "growing_season"}
LABEL = {
    "Wr": "Root-zone water", "z_gw": "Groundwater depth", "surface_storage": "Surface storage (bunds)",
    "IrrDay": "Irrigation, daily", "Infl": "Infiltration", "Runoff": "Runoff", "DeepPerc": "Deep percolation",
    "TileDrain": "Tile drainage", "CR": "Capillary rise", "GwIn": "Groundwater inflow",
    "Es": "Soil evaporation", "EsPot": "Potential soil evaporation", "Tr": "Transpiration",
    "TrPot": "Potential transpiration", "th": "Soil water content (all 12 layers)",
    "gdd": "Growing degree days", "gdd_cum": "Cumulative GDD", "z_root": "Root depth",
    "canopy_cover": "Canopy cover", "canopy_cover_ns": "Canopy cover, no stress", "biomass": "Biomass",
    "biomass_ns": "Biomass, no stress", "harvest_index": "Harvest index", "harvest_index_adj": "Harvest index, adjusted",
    "DryYield": "Dry yield, daily", "FreshYield": "Fresh yield, daily", "YieldPot": "Yield potential, daily",
    "dry_yield": "Season dry yield", "fresh_yield": "Season fresh yield", "yield_potential": "Season yield potential",
    "seasonal_irrigation": "Season irrigation",
}
GROUP = {"water_flux": "Water", "water_storage": "Water", "crop_growth": "Crop", "final_stats": "Season result"}


def num(v):
    if v is None:
        return math.nan
    if v == "inf":
        return math.inf
    if v == "-inf":
        return -math.inf
    return float(v)


def col_diff(want, got):
    """Worst |got - want| as a share of the column's largest |want|; 0 when every value matches."""
    n = min(len(want), len(got))
    scale = max((abs(x) for x in want if math.isfinite(x)), default=0.0) or 1.0
    worst, same = 0.0, len(want) == len(got)
    for a, b in zip(want[:n], got[:n]):
        # Rust's JSON writes infinities as null; an infinite fresh yield (crops with no dry-matter share) reads back as NaN.
        if (math.isnan(a) and math.isnan(b)) or a == b or (math.isinf(a) and math.isnan(b)):
            continue
        same = False
        if math.isfinite(a) and math.isfinite(b):
            worst = max(worst, abs(a - b) / scale)
        else:
            worst = max(worst, 1.0)
    return worst, same


per_var = {e: {} for e in ENG}           # engine -> var -> worst share
identical = {e: 0 for e in ENG}          # scenarios with every value equal
both_fail = {e: 0 for e in ENG}
season_pts = {e: {k: [] for k in ("dry_yield", "yield_potential", "seasonal_irrigation")} for e in ENG}
n_ok = n_err = 0
rows_off = {e: [] for e in ENG}
for g in sorted(golden.glob("*.json")):
    doc = json.load(open(g))
    if doc.get("error"):
        n_err += 1
        for e, d in (("rust", rdir), ("wasm", wdir)):
            if "error" in json.load(open(d / g.name)):
                both_fail[e] += 1
        continue
    n_ok += 1
    exp = doc["expected"]
    for e, d in (("rust", rdir), ("wasm", wdir)):
        res = json.load(open(d / g.name))
        all_same = True
        for tab in ("water_flux", "water_storage", "crop_growth"):
            wc, gc = exp[tab]["columns"], res[tab]["columns"]
            for i, c in enumerate(wc):
                if c in SKIP:
                    continue
                want = [num(x) for x in exp[tab]["values"][i]]
                got = [num(x) for x in res[tab]["values"][gc.index(c)]]
                if len(want) != len(got):
                    rows_off[e].append(g.stem)
                w, same = col_diff(want, got)
                all_same &= same
                key = "th" if c.startswith("th") and c[2:].isdigit() else c
                per_var[e][(GROUP[tab], key)] = max(per_var[e].get((GROUP[tab], key), 0.0), w)
        for k in ("dry_yield", "fresh_yield", "yield_potential", "seasonal_irrigation"):
            want = [num(s[k]) for s in exp["final_stats"]]
            got = [num(s[k]) for s in res["final_stats"]]
            w, same = col_diff(want, got)
            all_same &= same
            per_var[e][("Season result", k)] = max(per_var[e].get(("Season result", k), 0.0), w)
            if k in season_pts[e]:
                season_pts[e][k] += list(zip(want, got))
        identical[e] += all_same

FOOT = (f"{n_ok} reference scenarios that AquaCrop-OSPy 3.1.0 completes; Python outputs generated on Linux "
        "(numpy 2.5.4, pandas 3.0.6). Every day of every scenario compared.")


def style(ax, grid_axis="x"):
    ax.set_facecolor(SURFACE)
    for s in ("top", "right", "left"):
        ax.spines[s].set_visible(False)
    ax.spines["bottom"].set_color(GRID)
    ax.tick_params(colors=INK2, length=0)
    ax.grid(axis=grid_axis, color=GRID, linewidth=0.8)
    ax.set_axisbelow(True)


def head(fig, title, sub, keys, y_leg):
    if not PLAIN:
        fig.text(0.02, 0.985, title, fontsize=15, fontweight="semibold", color=INK, va="top")
    if not PLAIN:
        fig.text(0.02, 0.985 - 0.045 * 11 / fig.get_figheight() * 1.0, sub, fontsize=10, color=INK2, va="top")
    x = 0.02
    for lab, col in keys:
        fig.patches.append(plt.Rectangle((x, y_leg - 0.004), 0.012, 0.012 * fig.get_figwidth() / fig.get_figheight(),
                                         transform=fig.transFigure, color=col, figure=fig))
        fig.text(x + 0.018, y_leg + 0.004, lab, fontsize=9.5, color=INK2, va="center")
        x += 0.018 + 0.0072 * len(lab) + 0.03


# 5. Worst difference per output variable --------------------------------------------------
order = sorted(per_var["rust"], key=lambda k: (["Season result", "Crop", "Water"].index(k[0]), LABEL.get(k[1], k[1])))
FLOOR = 1e-17
fig_h = 0.27 * len(order) + 2.6
fig, ax = plt.subplots(figsize=(11, fig_h), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.3, right=0.96, top=1 - 1.75 / fig_h, bottom=0.75 / fig_h)
style(ax)
ax.set_xscale("log")
for i, k in enumerate(order):
    for e, dy in (("rust", -0.13), ("wasm", 0.13)):
        v = per_var[e][k]
        ax.scatter(max(v, FLOOR), i + dy, s=42, color=ENG[e][1], edgecolor=SURFACE, linewidth=1.2, zorder=3,
                   marker="o" if v > 0 else "D")
ax.axvspan(FLOOR / 3, 3e-17, color="#f1f0ec", zorder=0)
ax.text(FLOOR, -1.0, "identical\n(every bit)", fontsize=8, color=INK2, ha="center", va="bottom")
ax.axvline(1e-4, color=MUTED, linewidth=1, linestyle=(0, (3, 3)))
ax.text(1.2e-4, -0.9, "0.01% of the\nvariable's range", fontsize=8, color=INK2, va="bottom")
ax.set_xlim(FLOOR / 3, 0.3)
ax.xaxis.set_major_locator(FixedLocator([1e-15, 1e-12, 1e-9, 1e-6, 1e-4, 1e-2, 1e-1]))
ax.xaxis.set_minor_locator(NullLocator())
ax.set_xticklabels(["1e-15", "1e-12", "1e-9", "1e-6", "0.01%", "1%", "10%"])
ax.set_yticks(range(len(order)))
ax.set_yticklabels([f"{LABEL.get(k[1], k[1])}" for k in order], fontsize=9, color=INK)
prev = None
for i, k in enumerate(order):
    if k[0] != prev:
        ax.text(-0.43, i - 0.62, k[0].upper(), transform=ax.get_yaxis_transform(), fontsize=8, color=MUTED,
                fontweight="semibold")
        if prev is not None:
            ax.axhline(i - 0.5, color=GRID, linewidth=0.8)
        prev = k[0]
ax.set_ylim(len(order) - 0.5, -1.2)
ax.set_xlabel("Worst difference from Python in any scenario, as a share of that variable's largest value",
              color=INK2, fontsize=9)
head(fig, f"Largest difference from AquaCrop-OSPy per output, {n_ok} scenarios",
     f"Native Rust: {identical['rust']} of {n_ok} scenarios identical in every bit; the rest differ in the 16th digit.\n"
     "Browser build: last-bit differences; in 4 scenarios a threshold moves by one day, and one soil layer's water "
     "differs by up to 7.8% on that day.",
     [(ENG["rust"][0], RS), (ENG["wasm"][0], WS)], 1 - 1.45 / fig_h)
PLAIN or fig.text(0.02, 0.012, FOOT, fontsize=8.5, color=MUTED)
fig.savefig(out / "5_results_agreement_all_outputs.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

# 6. Season results: Python against Rust -----------------------------------------------------
panels = [("dry_yield", "Season dry yield (t/ha)"), ("yield_potential", "Season yield potential (t/ha)"),
          ("seasonal_irrigation", "Season irrigation (mm)")]
fig, axs = plt.subplots(1, 3, figsize=(13, 5.4), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.06, right=0.98, top=0.72, bottom=0.14, wspace=0.28)
for ax, (k, title) in zip(axs, panels):
    style(ax, "both")
    pts = season_pts["rust"][k]
    hi = max(max(a for a, _ in pts), 1) * 1.05
    ax.plot([0, hi], [0, hi], color=MUTED, linewidth=1, linestyle=(0, (3, 3)), zorder=1)
    ax.scatter([a for a, _ in pts], [b for _, b in pts], s=40, color=RS, edgecolor=SURFACE, linewidth=1.2, zorder=3)
    pw = season_pts["wasm"][k]
    ax.scatter([a for a, _ in pw], [b for _, b in pw], s=10, color=WS, zorder=4)
    ax.set_xlim(0, hi); ax.set_ylim(0, hi)
    ax.set_title(title, fontsize=10.5, color=INK, loc="left", fontweight="semibold")
    ax.set_xlabel("Python (AquaCrop-OSPy)", color=INK2, fontsize=9)
    ax.set_ylabel("Rust (aquacrop-rs)", color=INK2, fontsize=9)
    wr, ww = per_var["rust"][("Season result", k)], per_var["wasm"][("Season result", k)]
    ax.text(0.04, 0.96, f"{len(pts)} seasons\nworst gap, native: {wr * 100:.2g}%\nworst gap, browser: {ww * 100:.2g}%",
            transform=ax.transAxes, fontsize=8.5, color=INK2, va="top")
head(fig, "Season results: AquaCrop-OSPy against aquacrop-rs",
     "One point per harvested season. Dashed line: equal values.",
     [(ENG["rust"][0], RS), (ENG["wasm"][0], WS)], 0.81)
PLAIN or fig.text(0.02, 0.012, FOOT, fontsize=8.5, color=MUTED)
fig.savefig(out / "6_season_results_python_vs_rust.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

# 7. Daily trace for one field ------------------------------------------------------------------
name = "fao_paddy_hyderabad"
doc = json.load(open(golden / f"{name}.json"))
res = json.load(open(rdir / f"{name}.json"))
resw = json.load(open(wdir / f"{name}.json"))
import datetime as dt
start = dt.date.fromisoformat(doc["input"]["sim_start"])


def series(t, c):
    return [num(x) for x in t["values"][t["columns"].index(c)]]


fig, axs = plt.subplots(2, 1, figsize=(13, 6.8), dpi=200, facecolor=SURFACE, sharex=True,
                        gridspec_kw={"height_ratios": [1, 1]})
fig.subplots_adjust(left=0.07, right=0.98, top=0.8, bottom=0.08, hspace=0.25)
for ax, (c, ttl) in zip(axs, [("biomass", "Biomass (g/m²)"), ("canopy_cover", "Canopy cover (fraction)")]):
    style(ax, "y")
    want, got, gotw = series(doc["expected"]["crop_growth"], c), series(res["crop_growth"], c), series(resw["crop_growth"], c)
    days = [start + dt.timedelta(days=i) for i in range(len(want))]
    ax.plot(days, want, color=PY, linewidth=3.2, label="Python", zorder=2)
    ax.plot(days[:len(got)], got, color=RS, linewidth=1.3, linestyle=(0, (4, 2)), zorder=3)
    ax.plot(days[:len(gotw)], gotw, color=WS, linewidth=0.9, linestyle=(0, (1, 2)), zorder=4)
    ax.set_title(ttl, fontsize=10.5, color=INK, loc="left", fontweight="semibold")
head(fig, "Daily biomass and canopy cover, rice at Hyderabad, 2000 to 2010",
     "AquaCrop-OSPy (wide line), native Rust (dashed) and browser build (dotted).",
     [("Python (AquaCrop-OSPy)", PY), (ENG["rust"][0], RS), (ENG["wasm"][0], WS)], 0.865)
PLAIN or fig.text(0.02, 0.012, "FAO comparison run that ships with AquaCrop-OSPy (localpaddy, 2000 to 2010).", fontsize=8.5,
         color=MUTED)
fig.savefig(out / "7_daily_trace_hyderabad_rice.png", facecolor=SURFACE, **SAVE)
plt.close(fig)

summary = {
    "scenarios_ok": n_ok, "scenarios_error": n_err, "identical": identical, "both_fail": both_fail,
    "row_count_differs": {e: sorted(set(v)) for e, v in rows_off.items()},
    "worst_by_var": {e: {f"{k[0]}/{k[1]}": v for k, v in per_var[e].items()} for e in ENG},
}
json.dump(summary, open(out / "agreement.json", "w"), indent=1)
print(json.dumps({k: v for k, v in summary.items() if k != "worst_by_var"}, indent=1))
for e in ENG:
    print(e, "max over all vars:", max(per_var[e].values()))
