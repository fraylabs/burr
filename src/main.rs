use std::path::PathBuf;

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
         Usage:\n  burr <folder>\n\n\
         Run `burr .` to browse STEP, STL, and GLB files in the current project.\n\n\
         Options:\n  -h, --help       Show this help\n  -V, --version    Show the installed version"
    );
}
