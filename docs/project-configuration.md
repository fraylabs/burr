# Burr Project Configuration

`burr .` opens a local CAD project. Configuration is optional: without it,
Burr scans the directory passed on the command line.

Use `.burr/config.toml` when a project needs an explicit model scope or a named
motion between two assembly poses:

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

[[motions]]
id = "fold"
label = "Fold hanger"
from = "models/hanger-deployed.step"
from_label = "Deployed"
to = "models/hanger-folded.step"
to_label = "Folded"
duration_ms = 1200
```

The nearest `.burr/config.toml` found at or above the requested directory owns
the project. `project.models` is a required non-empty list of existing
directories relative to that project root.

Burr rejects absolute paths, parent traversal, symlink escapes, duplicate
roots, overlapping roots, and unknown configuration fields. Globs and check
configuration are not part of this contract.

Each optional `[[motions]]` entry connects two existing STEP files inside the
configured model scope. Both poses must contain the same uniquely named
assembly components with unchanged component geometry; Burr interpolates only
their rigid transforms. A motion supports up to 32 components and a duration
from 100 to 10,000 milliseconds. Selecting either endpoint reveals a labelled
playback scrubber, and **Snapshot** captures the currently displayed pose.

While Burr is running, `GET /api/project` returns the project name,
configuration state, portable model paths, and named motions. It does not expose
machine-specific absolute paths.
