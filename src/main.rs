use burr::{
    build_receipt_repair_packet, build_repair_report_packet, find_design_data_paths,
    format_receipt_diagnostics, format_receipt_explanations, init_project, lint_targets,
    stamp_targets, LintOptions, BURR_VERSION, DESIGN_DATA_FILE_NAME,
};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        Some("--version") | Some("-v") | Some("version") => {
            println!("{BURR_VERSION}");
            Ok(())
        }
        Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        None => {
            print_help();
            std::process::exit(2);
        }
        Some("check") => run_check(args.collect()),
        Some("explain") => run_explain(args.collect()),
        Some("stamp") => run_stamp(args.collect()),
        Some("init") => run_init(args.collect()),
        Some(command) => Err(format!("Unknown command: {command}")),
    }
}

fn run_init(args: Vec<String>) -> Result<(), String> {
    if args.len() != 1 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        std::process::exit(if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            0
        } else {
            2
        });
    }

    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let project_dir = PathBuf::from(&args[0]);
    let written = init_project(&project_dir)?;
    println!("INIT {}", relative_label(&cwd, &project_dir));
    for path in written {
        println!("WRITE {}", relative_label(&cwd, &path));
    }
    println!();
    println!("Next:");
    println!("  cd {}", relative_label(&cwd, &project_dir));
    println!("  uv run python design.py");
    println!("  burr check .");
    Ok(())
}

fn run_check(args: Vec<String>) -> Result<(), String> {
    let options = parse_check_args(args)?;
    if options.help {
        print_help();
        return Ok(());
    }
    if options.inputs.is_empty() {
        print_help();
        std::process::exit(2);
    }

    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let lint_options = LintOptions {
        rulepack_path: options.rulepack_path,
        write_receipt: options.write_receipt,
        cwd: cwd.clone(),
    };
    let results = lint_targets(&options.inputs, &lint_options)?;
    let mut failures = 0;

    for result in results {
        let status = result
            .receipt
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("fail");
        if status == "fail" {
            failures += 1;
        }
        let receipt_label = if options.write_receipt {
            relative_label(&cwd, &result.receipt_path)
        } else {
            "<not written>".to_string()
        };
        println!(
            "{} {} -> {}",
            status.to_uppercase(),
            relative_label(&cwd, &result.design_data_path),
            receipt_label
        );

        let diagnostics = format_receipt_diagnostics(&result.receipt);
        if !diagnostics.is_empty() {
            println!();
            println!(
                "{} problem{}:",
                diagnostics.len(),
                if diagnostics.len() == 1 { "" } else { "s" }
            );
            for (index, lines) in diagnostics.iter().enumerate() {
                if let Some(first) = lines.first() {
                    println!("{}. {first}", index + 1);
                }
                for line in lines.iter().skip(1) {
                    println!("   {line}");
                }
            }
            println!();
        }
    }

    std::process::exit(if failures == 0 { 0 } else { 1 });
}

fn run_explain(args: Vec<String>) -> Result<(), String> {
    let options = parse_explain_args(args)?;
    if options.help {
        print_help();
        return Ok(());
    }
    if options.inputs.is_empty() {
        print_help();
        std::process::exit(2);
    }

    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let mut json_outputs = Vec::new();
    for input in options.inputs {
        let path = resolve_explain_path(PathBuf::from(input));
        let document = read_json_document(&path)?;
        if options.json {
            json_outputs.push(explain_json_packet(&document));
            continue;
        }

        if string_field(&document, "schema_version") == Some("burr.repair-report.v1") {
            return Err(
                "Human explain currently expects a receipt. Use --json for repair reports."
                    .to_string(),
            );
        }
        println!("EXPLAIN {}", relative_label(&cwd, &path));
        if let Some(source) = document
            .get("source_design_data")
            .and_then(serde_json::Value::as_str)
        {
            println!("Source: {source}");
        }
        let status = document
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        println!("Status: {status}");

        let explanations = format_receipt_explanations(&document);
        if explanations.is_empty() {
            println!("No failed checks in this receipt.");
            println!();
            continue;
        }

        println!();
        println!(
            "{} failed check{}:",
            explanations.len(),
            if explanations.len() == 1 { "" } else { "s" }
        );
        for (index, lines) in explanations.iter().enumerate() {
            println!(
                "{}. {}",
                index + 1,
                lines.first().unwrap_or(&"Failure".to_string())
            );
            for line in lines.iter().skip(1) {
                println!("   {line}");
            }
        }
        println!();
    }

    if options.json {
        let output = if json_outputs.len() == 1 {
            json_outputs.remove(0)
        } else {
            serde_json::json!({
                "schema_version": "burr.repair-packet-list.v1",
                "burr_version": BURR_VERSION,
                "packets": json_outputs
            })
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .map_err(|error| format!("Failed to serialize repair packet JSON: {error}"))?
        );
    }

    Ok(())
}

fn run_stamp(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() {
        print_help();
        std::process::exit(2);
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let paths = find_design_data_paths(&args, &cwd)?;
    if paths.is_empty() {
        return Err(format!("No {DESIGN_DATA_FILE_NAME} files found."));
    }
    for path in stamp_targets(&args, &cwd)? {
        println!("STAMP {}", relative_label(&cwd, &path));
    }
    Ok(())
}

struct ParsedCheckArgs {
    inputs: Vec<String>,
    rulepack_path: Option<PathBuf>,
    write_receipt: bool,
    help: bool,
}

struct ParsedExplainArgs {
    inputs: Vec<String>,
    json: bool,
    help: bool,
}

fn parse_check_args(args: Vec<String>) -> Result<ParsedCheckArgs, String> {
    let mut inputs = Vec::new();
    let mut rulepack_path = None;
    let mut write_receipt = true;
    let mut help = false;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--rulepack" => {
                let Some(path) = iter.next() else {
                    return Err("--rulepack requires a file path.".to_string());
                };
                rulepack_path = Some(PathBuf::from(path));
            }
            "--no-write-receipt" => write_receipt = false,
            "--help" | "-h" => help = true,
            unknown if unknown.starts_with("--") => {
                return Err(format!("Unknown argument: {unknown}"));
            }
            _ => inputs.push(arg),
        }
    }

    Ok(ParsedCheckArgs {
        inputs,
        rulepack_path,
        write_receipt,
        help,
    })
}

fn parse_explain_args(args: Vec<String>) -> Result<ParsedExplainArgs, String> {
    let mut inputs = Vec::new();
    let mut json = false;
    let mut help = false;

    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => help = true,
            unknown if unknown.starts_with("--") => {
                return Err(format!("Unknown argument: {unknown}"));
            }
            _ => inputs.push(arg),
        }
    }

    Ok(ParsedExplainArgs { inputs, json, help })
}

fn resolve_explain_path(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.join("burr-receipt.json")
    } else {
        path
    }
}

fn read_json_document(path: &std::path::Path) -> Result<serde_json::Value, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse {} as JSON: {error}", path.display()))
}

fn explain_json_packet(document: &serde_json::Value) -> serde_json::Value {
    if string_field(document, "schema_version") == Some("burr.repair-report.v1") {
        build_repair_report_packet(document)
    } else {
        build_receipt_repair_packet(document)
    }
}

fn print_help() {
    println!(
        "Usage:\n  burr init <folder>\n  burr check [--rulepack <file>] [--no-write-receipt] <folder|{DESIGN_DATA_FILE_NAME}>...\n  burr explain [--json] <folder|burr-receipt.json|repair-report.json>...\n  burr stamp <folder|{DESIGN_DATA_FILE_NAME}>...\n"
    );
}

fn string_field<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn relative_label(cwd: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(cwd)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}
