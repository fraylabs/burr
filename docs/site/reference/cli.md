# CLI Reference

Burr ships as a Rust CLI named `burr`.

Install:

```bash
cargo install burr --version 0.30.0
```

## Commands

```txt
burr --version
burr <folder>
burr init <folder>
burr check [--rulepack <selector>] [--no-write-receipt] <folder|burr-design-data.json>...
burr explain [--json] <folder|burr-receipt.json|repair-report.json>...
burr stamp <folder|burr-design-data.json>...
```

## `burr <folder>`

Open a local browser for the STEP, STL, and GLB files below `folder`.

```bash
burr .
burr models
```

The sidebar preserves nested folders while hiding unrelated files and common
build directories. Burr renders with Look on the local machine and refreshes
the active model when its source changes. Without project configuration, no
`models/` convention is required; passing `models` simply limits the browser to
that subtree.

When the nearest project root contains `.burr/config.toml`, Burr scans only the
declared `project.models` directories and deterministically resolves every
configured built-in or local pack before opening the browser. Missing
configuration retains the zero-configuration folder behavior and enables no
packs. Invalid configuration stops startup with an explicit error.

## `burr init`

Creates a minimal `build123d` starter project:

```bash
burr init my-part
cd my-part
uv sync
uv run python design.py
burr check .
```

The generated design data explicitly selects `builtin:actuator_mount`; the
starter does not depend on an implicit default.

## `burr check`

Runs the linter:

```txt
find burr-design-data.json
  -> verify supported schema versions
  -> verify source and artifact hashes
  -> require and load an explicitly selected rulepack
  -> validate rulepack compatibility and contract
  -> check declared features
  -> report warnings and checked/unchecked coverage
  -> write burr-receipt.json
```

Use `--no-write-receipt` when a caller only wants terminal output. Select a
rulepack in design data or with `--rulepack`; Burr has no implicit fallback.
Both built-in and file selectors are supported:

```bash
burr check --rulepack builtin:actuator_mount .
burr check --rulepack rules/printed_plate.rulepack.json .
```

The command prints receipt warnings and checked/unchecked feature coverage. Its
exit code follows the trust outcome:

| Outcome | Exit | Meaning |
| --- | ---: | --- |
| `pass` | `0` | The selected rulepack was compatible and evaluated checks passed with complete required mechanical coverage. |
| `fail` | `1` | A checked claim failed or the rulepack contract is invalid. |
| invocation/configuration error | `2` | Burr could not read or select the requested inputs. |
| `incomplete` | `3` | Burr ran, but could not establish complete required mechanical coverage. |

For multiple targets, any `fail` produces exit `1`; otherwise any `incomplete`
produces exit `3`. Invocation and read errors remain exit `2`.

## `burr explain`

Expands failed checks into fix guidance:

```bash
burr explain .
burr explain --json .
```

Human output is for review. JSON output is for agent repair loops. For an
`incomplete` receipt, both forms retain scope warnings; the JSON repair packet
also includes `scope`, `warnings`, and normalized `incomplete_reasons` so a
caller does not mistake an empty failure list for a passing result. Consumers
must require `burr.repair-packet.v2`; multi-input JSON uses
`burr.repair-packet-list.v2`.

## `burr stamp`

Updates declared source and artifact hashes in `burr-design-data.json`.
