//! Day-by-day comparison with AquaCrop-OSPy 3.1.0.
//!
//! `python scripts/golden.py` writes the reference runs into tests/golden/.
//! This test runs each through the port and compares every column of every
//! daily table and the season results. Set GOLDEN=name1,name2 to run a subset
//! and GOLDEN_VERBOSE=1 to print the first diverging days.

use aquacrop::{run, ModelInput};
use serde_json::Value;
use std::path::PathBuf;

const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-9;

fn close(a: f64, b: f64) -> bool {
    if a == b || a.is_nan() && b.is_nan() {
        return true;
    }
    (a - b).abs() <= ATOL + RTOL * b.abs()
}

fn as_f64(v: &Value) -> f64 {
    match v.as_str() {
        Some("inf") => f64::INFINITY,
        Some("-inf") => f64::NEG_INFINITY,
        _ => v.as_f64().unwrap_or(f64::NAN),
    }
}

struct Report {
    worst: f64,
    /// The earliest day of a difference beyond the tolerance, and what it was.
    first: Option<(usize, String)>,
    /// The earliest day any value differs at all, however little.
    first_inexact: Option<(usize, String)>,
    mismatches: usize,
    all: Vec<(usize, String)>,
}

impl Report {
    fn note(&mut self, day: usize, msg: String) {
        self.mismatches += 1;
        if self.first.as_ref().is_none_or(|(d, _)| day < *d) {
            self.first = Some((day, msg.clone()));
        }
        self.all.push((day, msg));
    }
}

fn compare_table(name: &str, got: &aquacrop::output::Table, want: &Value, r: &mut Report) {
    let cols: Vec<&str> = want["columns"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    let vals = want["values"].as_array().unwrap();
    for (ci, col) in cols.iter().enumerate() {
        let w: Vec<f64> = vals[ci].as_array().unwrap().iter().map(as_f64).collect();
        let Some(g) = got.column(col) else {
            r.note(0, format!("{name}.{col}: column missing"));
            continue;
        };
        if g.len() != w.len() {
            r.note(0, format!("{name}.{col}: {} rows, expected {}", g.len(), w.len()));
            continue;
        }
        for day in 0..w.len() {
            let (a, b) = (g[day], w[day]);
            let same = a == b || a.is_nan() && b.is_nan();
            if !same {
                let rel = (a - b).abs() / b.abs().max(1e-12);
                if (a - b).abs() > ATOL && rel > r.worst {
                    r.worst = rel;
                }
                if r.first_inexact.as_ref().is_none_or(|(d, _)| day < *d) {
                    r.first_inexact = Some((day, format!("{name}.{col}: got {a:?}, expected {b:?}")));
                }
            }
            if !close(a, b) {
                r.note(day, format!("{name}.{col} day {day}: got {a:e}, expected {b:e}"));
            }
        }
    }
}

#[test]
fn matches_aquacrop_ospy() {
    // Relative to the package root, where cargo runs tests (also under WASI).
    let dir = std::env::var("GOLDEN_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("tests/golden"));
    let mut files: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(d) => d.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect(),
        Err(_) => {
            eprintln!("no tests/golden directory; run `python scripts/golden.py` first");
            return;
        }
    };
    files.sort();
    let only: Option<Vec<String>> = std::env::var("GOLDEN").ok().map(|s| s.split(',').map(String::from).collect());
    let verbose = std::env::var("GOLDEN_VERBOSE").is_ok();
    let mut failures = vec![];
    let mut checked = 0;
    for f in files {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let name = doc["name"].as_str().unwrap().to_string();
        if let Some(o) = &only {
            if !o.contains(&name) {
                continue;
            }
        }
        checked += 1;
        let input: ModelInput = match serde_json::from_value(doc["input"].clone()) {
            Ok(i) => i,
            Err(e) => {
                failures.push(format!("{name}: input does not parse: {e}"));
                continue;
            }
        };
        let t0 = std::time::Instant::now();
        let result = run(&input);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if let Some(err) = doc["error"].as_str() {
            // Only AquaCrop's own assertions are model behaviour; other
            // Python errors are environment bugs the port need not copy.
            match (&result, err.starts_with("AssertionError")) {
                (Err(_), _) => eprintln!("ok   {name}: fails as AquaCrop-OSPy does ({})", err.lines().next().unwrap_or("")),
                (Ok(_), false) => eprintln!("ok   {name}: runs; AquaCrop-OSPy fails with a non-model error ({err})"),
                (Ok(_), true) => failures.push(format!("{name}: AquaCrop-OSPy fails with {err}, the port runs")),
            }
            continue;
        }
        let res = match result {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!("{name}: the port fails: {e}"));
                continue;
            }
        };
        let exp = &doc["expected"];
        let mut r = Report { worst: 0.0, first: None, first_inexact: None, mismatches: 0, all: vec![] };
        compare_table("water_flux", &res.water_flux, &exp["water_flux"], &mut r);
        compare_table("water_storage", &res.water_storage, &exp["water_storage"], &mut r);
        compare_table("crop_growth", &res.crop_growth, &exp["crop_growth"], &mut r);
        let want_fs = exp["final_stats"].as_array().unwrap();
        if want_fs.len() != res.final_stats.len() {
            r.note(usize::MAX, format!("final_stats: {} seasons, expected {}", res.final_stats.len(), want_fs.len()));
        } else {
            for (g, w) in res.final_stats.iter().zip(want_fs) {
                let ok = g.season as u64 == w["season"].as_u64().unwrap()
                    && g.harvest_date.to_string() == w["harvest_date"].as_str().unwrap()
                    && g.harvest_step as u64 == w["harvest_step"].as_u64().unwrap()
                    && close(g.dry_yield, as_f64(&w["dry_yield"]))
                    && close(g.fresh_yield, as_f64(&w["fresh_yield"]))
                    && close(g.yield_potential, as_f64(&w["yield_potential"]))
                    && close(g.seasonal_irrigation, as_f64(&w["seasonal_irrigation"]));
                if !ok {
                    r.note(usize::MAX, format!("final_stats season {}: got {g:?}, expected {w}", g.season));
                }
            }
        }
        let inexact = match &r.first_inexact {
            None => "bit-identical".to_string(),
            Some((d, m)) => format!("worst relative difference {:.1e}, first at day {d} ({m})", r.worst),
        };
        if r.mismatches == 0 {
            eprintln!("ok   {name}: {} seasons, {inexact}, {ms:.0} ms", res.final_stats.len());
        } else {
            let (_, first) = r.first.clone().unwrap_or_default();
            eprintln!("FAIL {name}: {} mismatches; earliest: {first}; {inexact}", r.mismatches);
            if verbose {
                r.all.sort();
                for (_, m) in r.all.iter().take(12) {
                    eprintln!("    {m}");
                }
            }
            failures.push(format!("{name}: {first}"));
        }
    }
    assert!(checked > 0 || only.is_some(), "no golden files found");
    if !failures.is_empty() {
        panic!("{} of {checked} scenarios differ from AquaCrop-OSPy:\n{}", failures.len(), failures.join("\n"));
    }
}
