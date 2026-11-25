# Changelog

## 0.1.0 (unreleased)

First version.

- AquaCrop-OSPy 3.1.0 ported to Rust: initialisation, all daily processes,
  multi-season runs, off-season simulation, every irrigation method, water
  tables, bunds and mulches, tile drainage, CO2 series, custom and
  texture-based soils, adaptive planting and the 37 built-in crops.
- WebAssembly build with `run`, `crop_names`, `crop_preset`, `soil_types`
  and `version`.
- Reference test against AquaCrop-OSPy on 74 scenarios, and a WebAssembly
  check against the same scenarios.
- Documentation, benchmarks and comparison charts.
