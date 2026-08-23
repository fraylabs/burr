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

The only built-in pack name recognized by this foundation is
`builtin:mechanical-fit`. It identifies the existing Burr design-check
capability that will be migrated onto the shared pack runtime. Configuration
resolution does not yet execute it.

## Local pack envelope

A local pack starts with a versioned identity envelope:

```toml
schema_version = "burr.pack.v1"
id = "project:product-fit"
version = "0.1.0"
```

Local packs must remain inside `.burr/`. Their future check definitions will be
handled by the shared pack contract; this foundation deliberately validates
only identity and deterministic resolution.

## Public project state

While Burr is running, `GET /api/project` returns only portable project-relative
paths, configuration state, model roots, and resolved pack identities. It does
not expose machine-specific absolute paths.
