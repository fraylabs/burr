# Burr Project Configuration

`burr .` opens a local CAD project. Configuration is optional: without it,
Burr scans the directory passed on the command line.

Use `.burr/config.toml` only when a project needs an explicit model scope:

```text
project/
├── models/
│   ├── enclosure.step
│   └── assembly.step
└── .burr/
    └── config.toml
```

```toml
schema_version = "burr.project.v1"

[project]
models = ["models"]
```

The nearest `.burr/config.toml` found at or above the requested directory owns
the project. `project.models` is a required non-empty list of existing
directories relative to that project root.

Burr rejects absolute paths, parent traversal, symlink escapes, duplicate
roots, overlapping roots, and unknown configuration fields. Globs and check
configuration are not part of this contract.

While Burr is running, `GET /api/project` returns the project name,
configuration state, and portable model paths. It does not expose
machine-specific absolute paths.
