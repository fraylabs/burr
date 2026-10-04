#[path = "../src/interference.rs"]
mod interference;
use look::{config::UpAxis, scene::compile_scene, timing::Timings};
use std::{path::Path, time::Instant};
fn main() {
    let file = std::env::args().nth(1).expect("STEP path");
    let start = Instant::now();
    let mut timings = Timings::default();
    match compile_scene(Path::new(&file), UpAxis::Z, &mut timings) {
        Ok(scene) => {
            let load = start.elapsed().as_secs_f64();
            eprintln!(
                "BENCH_LOADED {}",
                serde_json::json!({"load_s":load,"stages_ms":timings,"parts":scene.instances.len(),"geometries":scene.geometries.len(),"step_import":scene.statistics.step_import,"structure_errors":scene.assembly_structure_errors,"triangles":scene.geometries.iter().map(|g|g.indices.len()/3).sum::<usize>()})
            );
            let check = Instant::now();
            let report = interference::analyze_scene(&file, &scene.source_hash, &scene);
            println!(
                "{}",
                serde_json::json!({"load_s":load,"stages_ms":timings,"check_s":check.elapsed().as_secs_f64(),"parts":scene.instances.len(),"geometries":scene.geometries.len(),"step_import":scene.statistics.step_import,"structure_errors":scene.assembly_structure_errors,"triangles":scene.geometries.iter().map(|g|g.indices.len()/3).sum::<usize>(),"report":report})
            );
        }
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({"load_s":start.elapsed().as_secs_f64(),"error":format!("{e:#}")})
            );
            std::process::exit(2);
        }
    }
}
