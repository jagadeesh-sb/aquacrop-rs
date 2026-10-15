// Print the model state at one day of a golden scenario (debugging aid).
// cargo run --example debug_day -- tests/golden/<name>.json <day>
use aquacrop::solution::root_zone_water::root_zone_water;
use aquacrop::{Model, ModelInput};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let day: usize = args[2].parse().unwrap();
    let input: ModelInput = serde_json::from_value(doc["input"].clone()).unwrap();
    let mut m = Model::new(&input).unwrap();
    while m.clock.time_step_counter <= day && !m.is_finished() {
        m.step().unwrap();
    }
    let p = &m.params.Profile;
    println!("dzsum {:?}\ndz {:?}", p.dzsum, p.dz);
    println!("z_root {:?} th {:?}", m.state.z_root, &m.state.th[..8]);
    let sc = m.clock.season_counter.max(0) as usize;
    let crop = &m.params.Seasonal_Crop_List[sc];
    let rz = root_zone_water(p, m.state.z_root, &m.state.th, m.params.Soil.z_top, crop.Zmin, crop.Aer);
    println!("Zmin {} Aer {} WrAct {:?}", crop.Zmin, crop.Aer, rz.WrAct);
    println!("results Wr {:?}", m.results().water_flux.column("Wr").unwrap()[day]);
}
