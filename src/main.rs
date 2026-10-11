use std::path::PathBuf;

mod binary_viewer;
mod cache;
mod interference;
mod load_status;
mod motion;
mod project;
mod viewer;

fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("burr: {error}");
        std::process::exit(2);
    }
}

fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    match args.next().as_deref() {
        Some("--version" | "-V" | "version") => {
            reject_extra_args(args)?;
            println!(env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("--help" | "-h" | "help") => {
            reject_extra_args(args)?;
            print_help();
            Ok(())
        }
        Some("check") => {
            if args.next().as_deref() != Some("--json") {
                return Err("usage: burr check --json <assembly.step>".to_string());
            }
            let path = args.next().ok_or("provide a STEP assembly path")?;
            reject_extra_args(args)?;
            check_json(&PathBuf::from(path))
        }
        Some(path) => {
            reject_extra_args(args)?;
            let path = PathBuf::from(path);
            if !path.is_dir() {
                return Err(format!("model folder does not exist: {}", path.display()));
            }
            viewer::run(path)
        }
        None => {
            print_help();
            Err("provide a model folder, normally `burr .`".to_string())
        }
    }
}

fn check_json(path: &std::path::Path) -> Result<(), String> {
    let extension = path.extension().and_then(|v| v.to_str()).unwrap_or("");
    if !["step", "stp"].iter().any(|v| extension.eq_ignore_ascii_case(v)) {
        return Err("check requires a STEP or STP assembly".to_string());
    }
    let version = cache::source_fingerprint(path)?;
    let scene = look::scene::compile_scene(
        path, look::config::UpAxis::Z, &mut look::timing::Timings::default(),
    ).map_err(|error| format!("could not load {}: {error:#}", path.display()))?;
    let report = interference::analyze_scene_from_source(
        &path.to_string_lossy(), &version, &scene, path,
    );
    println!("{}", serde_json::to_string(&report).map_err(|e| e.to_string())?);
    match report.outcome {
        interference::CheckOutcome::Pass => Ok(()),
        interference::CheckOutcome::Fail => std::process::exit(1),
        interference::CheckOutcome::Incomplete => std::process::exit(3),
    }
}

fn reject_extra_args(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let Some(first) = args.next() else {
        return Ok(());
    };
    let mut extras = vec![first];
    extras.extend(args);
    Err(format!(
        "the model browser accepts one folder, but received: {}",
        extras.join(" ")
    ))
}

fn print_help() {
    println!(
        "Burr — local CAD model browser and assembly interference checker\n\n\
         Usage:\n  burr <folder>\n  burr check --json <assembly.step>\n\n\
         Run `burr .` to browse STEP, STL, and GLB files in the current project.\n\n\
         Options:\n  -h, --help       Show this help\n  -V, --version    Show the installed version"
    );
}
