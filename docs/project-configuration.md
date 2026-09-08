# Burr Project Configuration

`burr .` opens a local CAD project. Configuration is optional: without it,
Burr scans the directory passed on the command line.

Use `.burr/config.toml` when a project needs an explicit model scope or a named
rigid motion within one STEP assembly:

```text
project/
├── models/
│   ├── enclosure.step
│   └── assembly.step
└── .burr/
    └── config.toml
```

```toml
schema_version = "burr.project.v2"

[project]
models = ["models"]

[[motions]]
id = "fold"
label = "Fold hanger"
model = "models/hanger-deployed.step"
from_label = "Deployed"
to_label = "Folded"
duration_ms = 1200

[[motions.joints]]
type = "revolute"
components = ["left_arm"]
origin_mm = [-26.0, 0.0, 2.6]
axis = [0.0, 0.0, 1.0]
angle_degrees = 70.0

[[motions.joints]]
type = "revolute"
components = ["right_arm"]
origin_mm = [26.0, 0.0, 2.6]
axis = [0.0, 0.0, 1.0]
angle_degrees = -70.0

[[motions.joints]]
type = "prismatic"
components = ["lock_bar"]
axis = [0.0, 1.0, 0.0]
distance_mm = 7.5
```

The nearest `.burr/config.toml` found at or above the requested directory owns
the project. `project.models` is a required non-empty list of existing
directories relative to that project root.

Burr rejects absolute paths, parent traversal, symlink escapes, duplicate
roots, overlapping roots, and unknown configuration fields. Globs and check
configuration are not part of this contract.

Each optional `[[motions]]` entry references one existing STEP file inside the
configured model scope. That model is the pose named by `from_label`. Selecting
it reveals a labelled playback scrubber, and **Snapshot** captures the currently
displayed pose.

Each nested `[[motions.joints]]` table moves one or more exact STEP occurrence
names. Components omitted from every joint remain fixed. A component may belong
to only one joint in the current contract.

- A `revolute` joint rotates its components about `origin_mm` and `axis` by
  `angle_degrees` at the end of the motion.
- A `prismatic` joint translates its components along `axis` by `distance_mm`
  at the end of the motion.

Joint coordinates use the STEP model's millimetre coordinate system before
Burr's viewer-axis normalization. Axes are normalized automatically and must be
finite and non-zero. Revolute angles must be non-zero and no greater than 360
degrees in magnitude. Components grouped in one joint move together along the
same physical path.

A motion supports STEP assemblies with up to 32 named component occurrences
and a duration from 100 to 10,000 milliseconds. Burr tessellates `model` once,
samples the configured hinge arcs and translations at 60 frames per second,
and sends only the resulting transforms alongside the reusable mesh.

The current interference check evaluates the untransformed source assembly.
While a motion is playing or paused away from its source pose, Burr does not
show that source result as if it described the visible pose. Move the scrubber
back to `from_label` to run the check. Checks for animated poses remain future
work.

`burr.project.v1` used `from` and `to` STEP snapshots. Version 2 deliberately
replaces that format: set `model` to the former starting pose and describe the
actual pivots, axes and travel as joints. Burr cannot infer a trustworthy hinge
from two snapshots, so old configurations fail with an explicit schema-version
message instead of guessing.

While Burr is running, `GET /api/project` returns the project name,
configuration state, portable model paths, and named motions through
`burr.project-state.v2`. It does not expose joint internals or machine-specific
absolute paths.
