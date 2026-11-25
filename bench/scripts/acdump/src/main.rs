// Writes aquacrop-rs full results for each golden file: acdump <out_dir> <file.json>...
fn main() {
    let args: Vec<String> = std::env::args().collect();
    for path in &args[2..] {
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let input: aquacrop::ModelInput = serde_json::from_value(doc["input"].clone()).unwrap();
        let name = std::path::Path::new(path).file_stem().unwrap().to_str().unwrap();
        let body = match aquacrop::run(&input) {
            Ok(r) => serde_json::to_string(&r).unwrap(),
            Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
        };
        std::fs::write(format!("{}/{}.json", args[1], name), body).unwrap();
    }
}
