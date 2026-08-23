# Burr Project Configuration

`burr .` opens a local CAD project. A `.burr/config.toml` file makes the model
scope and enabled rule packs explicit. Without that file, Burr remains a
zero-configuration model browser rooted at the directory passed on the command
line and enables no project packs.

## Project layout

```text
project/
├── models/
│   ├── enclosure.step
│   └── assembly.step
└── .burr/
    ├── config.toml
    └── packs/
        └── product-fit.toml
```

The nearest `.burr/config.toml` found at or above the requested directory owns
the project. This permits running `burr .` from a configured subdirectory while
keeping paths relative to one stable project root.

## Configuration contract

```toml
schema_version = "burr.project.v1"

[project]
models = ["models"]

[[packs]]
id = "builtin:mechanical-fit"

[[packs]]
path = "packs/product-fit.toml"
```

`project.models` is a required non-empty list of existing directories relative
to the project root. Burr rejects absolute paths, parent traversal, symlink
escapes, duplicate roots, and overlapping roots. Globs are not part of the V1
contract.

Each `[[packs]]` entry declares exactly one source:

- `id` selects a pack bundled with the installed Burr version;
- `path` selects a local pack relative to `.burr/`.

Pack order is declaration order. Pack ids must be unique after resolution.
Unknown built-in packs, missing local files, duplicate ids, and invalid pack
metadata stop startup rather than silently disabling a requested check.

The first built-in project pack is `builtin:mechanical-fit`. At workbench
startup it finds `burr-design-data.json` files under the configured model roots
and runs the existing Burr mechanical evaluator without writing receipts. Each
design-data file still selects its specific mechanical rulepack; the project
pack is the orchestration layer that runs those declared checks together.

## Local pack envelope

A local pack starts with a versioned identity envelope:

```toml
schema_version = "burr.pack.v1"
id = "project:product-fit"
version = "0.1.0"
```

Local packs must remain inside `.burr/`. Burr currently validates their identity
and deterministic resolution but does not execute local check definitions. A
configured local pack therefore reports `incomplete` with
`local_pack_runtime_unavailable`; it never produces a false pass.

## Check runtime

Every configured pack produces one of three outcomes:

- `pass`: every target evaluated by the pack passed;
- `incomplete`: the requested pack could not establish a complete result;
- `fail`: at least one evaluated target failed.

Pack results declare the capabilities they require and the capabilities Burr
could supply. The V1 vocabulary is `mesh`, `brep`, `assembly`, and
`declared_intent`. The mechanical-fit adapter currently requires and supplies
`declared_intent`; later geometry packs can use the same contract without
inventing another result shape.

`GET /api/checks` returns `burr.check-results.v1` with the aggregate outcome,
per-pack outcomes, portable target paths, counts, and unified findings. Findings
include a stable code, severity, structured evidence, remediation when known,
and optional model/part/feature/face references for future viewer selection.
With no enabled packs, the aggregate outcome is `null`, not `pass`.

Workbench checks are a read-only startup snapshot. They do not create or update
`burr-receipt.json`. `burr check` remains the receipt-writing headless interface
and retains its existing exit-code contract. Automatic check reruns on watched
file changes and the visual checks panel are separate follow-up work.

## Public project state

While Burr is running, `GET /api/project` returns only portable project-relative
paths, configuration state, model roots, and resolved pack identities.
`GET /api/checks` follows the same portable-path rule. Neither endpoint exposes
machine-specific absolute paths.
