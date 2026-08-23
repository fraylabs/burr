use burr::{
    build_receipt_repair_packet, build_repair_report_packet, find_design_data_paths,
    format_receipt_diagnostics, format_receipt_explanations, init_project, lint_targets,
    stamp_targets, LintOptions, BURR_VERSION, DESIGN_DATA_FILE_NAME,
    REPAIR_PACKET_LIST_SCHEMA_VERSION,
};
use std::path::PathBuf;

mod project;
mod viewer;

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
        Some(path) => {
            let path = PathBuf::from(&path);
            if !path.is_dir() {
                return Err(format!(
                    "Unknown command or viewer folder: {}",
                    path.display()
                ));
            }
            let remaining = args.collect::<Vec<_>>();
            if !remaining.is_empty() {
                return Err(format!(
                    "The model viewer accepts one folder, but received: {}",
                    remaining.join(" ")
                ));
            }
            viewer::run(path)
        }
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
    let mut incompletes = 0;

    for result in results {
        let outcome = result
            .receipt
            .get("outcome")
            .or_else(|| result.receipt.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("fail");
        match outcome {
            "fail" => failures += 1,
            "incomplete" => incompletes += 1,
            "pass" => {}
            _ => failures += 1,
        }
        let receipt_label = if options.write_receipt {
            relative_label(&cwd, &result.receipt_path)
        } else {
            "<not written>".to_string()
        };
        println!(
            "{} {} -> {}",
            outcome.to_uppercase(),
            relative_label(&cwd, &result.design_data_path),
            receipt_label
        );

        print_receipt_scope_and_warnings(&result.receipt);

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

    std::process::exit(if failures > 0 {
        1
    } else if incompletes > 0 {
        3
    } else {
        0
    });
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
            .get("outcome")
            .or_else(|| document.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        println!("Status: {status}");
        print_receipt_scope_and_warnings(&document);

        let explanations = format_receipt_explanations(&document);
        if explanations.is_empty() {
            if status == "incomplete" {
                println!(
                    "No failed or incomplete check records; the incomplete outcome is explained by the scope information or warnings above."
                );
            } else {
                println!("No failed checks in this receipt.");
            }
            println!();
            continue;
        }

        println!();
        if status == "fail" {
            println!(
                "{} failed check{}:",
                explanations.len(),
                if explanations.len() == 1 { "" } else { "s" }
            );
        } else {
            println!(
                "{} check{} requiring attention:",
                explanations.len(),
                if explanations.len() == 1 { "" } else { "s" }
            );
        }
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
                "schema_version": REPAIR_PACKET_LIST_SCHEMA_VERSION,
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
                    return Err("--rulepack requires a file path or built-in selector.".to_string());
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
        "Usage:\n  burr <folder>\n  burr init <folder>\n  burr check [--rulepack <selector>] [--no-write-receipt] <folder|{DESIGN_DATA_FILE_NAME}>...\n  burr explain [--json] <folder|burr-receipt.json|repair-report.json>...\n  burr stamp <folder|{DESIGN_DATA_FILE_NAME}>...\n\nRun `burr .` to open the local STEP, STL, and GLB model browser.\n\nRulepack selectors may be a file path or builtin:actuator_mount.\n\nExit codes for burr check:\n  0  pass\n  1  fail\n  2  invocation or configuration error\n  3  incomplete\n"
    );
}

fn print_receipt_scope_and_warnings(receipt: &serde_json::Value) {
    if let Some(scope) = receipt.get("scope") {
        let declared_rules = scope
            .pointer("/rules/declared")
            .and_then(serde_json::Value::as_u64);
        let evaluated_rules = scope
            .pointer("/rules/evaluated")
            .and_then(serde_json::Value::as_u64);
        let declared_features = scope
            .pointer("/mechanical_features/declared")
            .and_then(serde_json::Value::as_u64);
        let checked_features = scope
            .pointer("/mechanical_features/checked")
            .and_then(serde_json::Value::as_u64);
        if let (
            Some(declared_rules),
            Some(evaluated_rules),
            Some(declared_features),
            Some(checked_features),
        ) = (
            declared_rules,
            evaluated_rules,
            declared_features,
            checked_features,
        ) {
            println!(
                "Scope: {evaluated_rules}/{declared_rules} rules evaluated; {checked_features}/{declared_features} mechanical features checked."
            );
        }

        let design_artifact = scope
            .pointer("/artifact_type/design")
            .and_then(serde_json::Value::as_str);
        let rulepack_artifact = scope
            .pointer("/artifact_type/rulepack")
            .and_then(serde_json::Value::as_str);
        let artifact_compatible = scope
            .pointer("/artifact_type/compatible")
            .and_then(serde_json::Value::as_bool);
        if artifact_compatible == Some(false) {
            println!(
                "Artifact scope: design={}, rulepack={} (not compatible).",
                design_artifact.unwrap_or("<missing>"),
                rulepack_artifact.unwrap_or("<missing>")
            );
        }

        let process_restricted = scope
            .pointer("/process_kind/restricted")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        let process_compatible = scope
            .pointer("/process_kind/compatible")
            .and_then(serde_json::Value::as_bool);
        if process_restricted && process_compatible == Some(false) {
            let design_process = scope
                .pointer("/process_kind/design")
                .and_then(serde_json::Value::as_str);
            let rulepack_process = scope
                .pointer("/process_kind/rulepack")
                .and_then(serde_json::Value::as_str);
            println!(
                "Process scope: design={}, rulepack={} (not compatible).",
                design_process.unwrap_or("<missing>"),
                rulepack_process.unwrap_or("<missing>")
            );
        }
    }

    let warnings: Vec<_> = receipt
        .get("warnings")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in warnings {
            let reason = string_field(warning, "reason").unwrap_or("warning");
            let message = string_field(warning, "message").unwrap_or("No detail provided.");
            let impact = if warning
                .get("affects_outcome")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                " [affects outcome]"
            } else {
                ""
            };
            println!("  - {reason}{impact}: {message}");
        }
    }
}

fn string_field<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn relative_label(cwd: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(cwd)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}
