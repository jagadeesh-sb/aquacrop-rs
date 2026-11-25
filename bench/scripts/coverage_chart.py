"""Line coverage per source file from `cargo llvm-cov --json`: coverage_chart.py <coverage.json> <out.png>"""
import json
import os
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import font_manager

# Poppins when AQ_FONT_DIR holds its .ttf files, otherwise matplotlib's default font.
_fonts = list(Path(os.environ.get("AQ_FONT_DIR", ".")).glob("Poppins-*.ttf"))
for f in _fonts:
    font_manager.fontManager.addfont(str(f))
PLAIN = os.environ.get("AQ_PLAIN") == "1"
plt.rcParams.update({"font.family": "Arial" if PLAIN else ("Poppins" if _fonts else "DejaVu Sans"), "font.size": 10})
SURFACE, INK, INK2, MUTED, GRID, BAR = "#fcfcfb", "#0b0b0b", "#52514e", "#8a8984", "#e6e5e1", "#eb6834"

data = json.load(open(sys.argv[1]))["data"][0]
tot = data["totals"]
rows = []
for f in data["files"]:
    name = f["filename"].split("/src/")[-1]
    s = f["summary"]["lines"]
    rows.append((name, s["percent"], s["covered"], s["count"]))
rows.sort(key=lambda r: (not r[0].startswith("solution/"), -r[1]))

h = 0.26 * len(rows) + 2.2
fig, ax = plt.subplots(figsize=(11, h), dpi=200, facecolor=SURFACE)
fig.subplots_adjust(left=0.27, right=0.95, top=1 - 1.45 / h, bottom=0.5 / h)
ax.set_facecolor(SURFACE)
for s in ("top", "right", "left"):
    ax.spines[s].set_visible(False)
ax.spines["bottom"].set_color(GRID)
ax.tick_params(colors=INK2, length=0)
ax.grid(axis="x", color=GRID, linewidth=0.8)
ax.set_axisbelow(True)
for i, (name, pct, cov, cnt) in enumerate(rows):
    ax.barh(i, pct, height=0.6, color=BAR)
    ax.text(pct + 0.8, i, f"{pct:.0f}%  ({cov}/{cnt} lines)", va="center", fontsize=8.5, color=INK)
ax.set_yticks(range(len(rows)))
ax.set_yticklabels([r[0] for r in rows], fontsize=8.5, color=INK)
ax.invert_yaxis()
ax.set_xlim(0, 118)
ax.set_xticks([0, 25, 50, 75, 100])
ax.set_xticklabels(["0%", "25%", "50%", "75%", "100%"])
L, F = tot["lines"], tot["functions"]
PLAIN or fig.text(0.02, 1 - 0.3 / h, f"Test coverage: {L['percent']:.1f}% of lines, {F['percent']:.1f}% of functions",
         fontsize=15, fontweight="semibold", color=INK, va="top")
PLAIN or fig.text(0.02, 1 - 0.78 / h, "Lines run by the unit tests and the 74-scenario reference test (cargo llvm-cov). "
         "Process modules first.", fontsize=10, color=INK2, va="top")
fig.savefig(sys.argv[2], facecolor=SURFACE, **({"bbox_inches": "tight", "pad_inches": 0.15} if PLAIN else {}))
