# CLI Reference

Burr ships as a Rust CLI named `burr`.

Install:

```bash
cargo install burr --version 0.21.0
```

## Commands

```txt
burr --version
burr init <folder>
burr check [--rulepack <file>] [--no-write-receipt] <folder|burr-design-data.json>...
burr explain [--json] <folder|burr-receipt.json|repair-report.json>...
burr stamp <folder|burr-design-data.json>...
```

## `burr init`

Creates a minimal `build123d` starter project:

```bash
burr init my-part
cd my-part
uv sync
uv run python design.py
burr check .
```

## `burr check`

Runs the linter:

```txt
find burr-design-data.json
  -> verify supported schema versions
  -> verify source and artifact hashes
  -> load selected rulepack
  -> check declared features
  -> write burr-receipt.json
```

Use `--no-write-receipt` when a caller only wants terminal output. Use
`--rulepack <file>` to override the rulepack declared by design data.

## `burr explain`

Expands failed checks into fix guidance:

```bash
burr explain .
burr explain --json .
```

Human output is for review. JSON output is for agent repair loops.

## `burr stamp`

Updates declared source and artifact hashes in `burr-design-data.json`.
