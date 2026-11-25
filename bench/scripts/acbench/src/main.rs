// Times aquacrop-rs on golden scenario files: acbench <reps> <file.json>...
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let reps: usize = args[1].parse().unwrap();
    for path in &args[2..] {
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let input: aquacrop::ModelInput = serde_json::from_value(doc["input"].clone()).unwrap();
        let mut times = Vec::new();
        let mut total_yield = f64::NAN;
        for _ in 0..reps.max(1) {
            let t = Instant::now();
            let r = aquacrop::run(&input);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            if let Ok(res) = r {
                let v = serde_json::to_value(&res).unwrap();
                total_yield = v["final_stats"].as_array().unwrap().iter()
                    .map(|s| s["dry_yield"].as_f64().unwrap_or(0.0)).sum();
            }
        }
        println!("{}", serde_json::json!({"file": path, "ms": times, "yield_sum": total_yield}));
    }
}
