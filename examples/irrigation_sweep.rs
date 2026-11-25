// Search irrigation schedules for wheat at Tunis, 1979 to 2002.
//
// Each schedule is a set of four soil-moisture thresholds (one per growth
// stage, AquaCrop-OSPy irrigation method 1). The sweep tries every
// combination of 0, 20, 40, 60 and 80 percent (625 schedules), runs the full
// 22-season simulation for each, and prints the schedules that give the most
// grain for the water applied.
//
//     cargo run --release --example irrigation_sweep
//     cargo run --release --example irrigation_sweep -- path/to/input.json
//
// The input file is an ordinary aquacrop-rs input; its irrigation settings
// are replaced by each schedule in turn.
use std::time::Instant;

const LEVELS: [f64; 5] = [0.0, 20.0, 40.0, 60.0, 80.0];

struct Outcome {
    smt: [f64; 4],
    yield_t_ha: f64,
    irrigation_mm: f64,
}

fn run_one(base: &aquacrop::ModelInput, smt: [f64; 4]) -> Outcome {
    let mut input = base.clone();
    input.irrigation = aquacrop::config::IrrigationSpec { irrigation_method: 1, SMT: Some(smt.to_vec()), ..Default::default() };
    let r = aquacrop::run(&input).expect("simulation runs");
    let n = r.final_stats.len().max(1) as f64;
    Outcome {
        smt,
        yield_t_ha: r.final_stats.iter().map(|s| s.dry_yield).sum::<f64>() / n,
        irrigation_mm: r.final_stats.iter().map(|s| s.seasonal_irrigation).sum::<f64>() / n,
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "examples/data/tunis_wheat.json".into());
    let base: aquacrop::ModelInput =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("input file")).expect("valid input");

    let mut schedules = Vec::new();
    for a in LEVELS {
        for b in LEVELS {
            for c in LEVELS {
                for d in LEVELS {
                    schedules.push([a, b, c, d]);
                }
            }
        }
    }

    // One thread.
    let t = Instant::now();
    let first = run_one(&base, schedules[0]);
    let one_run = t.elapsed().as_secs_f64();

    let t = Instant::now();
    let serial: Vec<Outcome> = schedules.iter().map(|&s| run_one(&base, s)).collect();
    let serial_s = t.elapsed().as_secs_f64();

    // All cores, plain std threads.
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let t = Instant::now();
    let chunk = schedules.len().div_ceil(threads);
    let parallel: Vec<Outcome> = std::thread::scope(|sc| {
        let handles: Vec<_> = schedules
            .chunks(chunk)
            .map(|part| sc.spawn(|| part.iter().map(|&s| run_one(&base, s)).collect::<Vec<_>>()))
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let parallel_s = t.elapsed().as_secs_f64();
    assert_eq!(parallel.len(), serial.len());

    let seasons = aquacrop::run(&base).unwrap().final_stats.len();
    println!("input: {path} ({seasons} seasons per run)");
    println!("schedules tried: {}", schedules.len());
    println!("one run: {:.1} ms", one_run * 1000.0);
    println!("all schedules, 1 thread: {serial_s:.2} s");
    println!("all schedules, {threads} threads: {parallel_s:.2} s");
    println!("rainfed (no irrigation): {:.2} t/ha", first.yield_t_ha);

    // Best yield within each irrigation budget.
    println!("\nbudget (mm/season)  best yield (t/ha)  water used (mm)  thresholds (%)");
    for budget in [0.0, 25.0, 50.0, 75.0, f64::INFINITY] {
        if let Some(best) = serial
            .iter()
            .filter(|o| o.irrigation_mm <= budget)
            .max_by(|a, b| a.yield_t_ha.total_cmp(&b.yield_t_ha).then(b.irrigation_mm.total_cmp(&a.irrigation_mm)))
        {
            let label = if budget.is_finite() { format!("{budget:.0}") } else { "no limit".into() };
            println!(
                "{label:>18}  {:>17.2}  {:>15.0}  {:?}",
                best.yield_t_ha, best.irrigation_mm, best.smt
            );
        }
    }
}
