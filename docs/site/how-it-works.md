# How Burr Works

Burr opens ordinary CAD files locally:

```txt
burr .
  -> discover STEP, STL, and GLB files
  -> preserve their folder hierarchy
  -> compile the selected model with Look
  -> render it in a local browser
  -> inspect STEP assembly component intersections
  -> refresh when the source file changes
```

No upload, generated manifest, or Burr-specific CAD helper is required for the
workbench.

## Optional project scope

Without configuration, the folder passed to `burr` is the model root. A project
can use `.burr/config.toml` to name one or more model folders while retaining a
stable project root:

```toml
schema_version = "burr.project.v1"

[project]
models = ["models"]
```

The project contract contains model scope only. It does not define rulepacks or
declared mechanical intent.

## Geometry-native assembly intersection

Look preserves each STEP assembly occurrence's component name, mesh, and world
transform. Burr checks those world-space component pairs with a strict bounds
filter followed by mesh interference and containment evidence. Face contact is
not an intersection.

The Checks tab reports `pass`, `fail`, or `incomplete`. Selecting a finding
highlights its two component occurrences in the same Look viewport. The result
is cached with the compiled model and is replaced when the source version
changes.

A clean result requires a STEP assembly with at least two closed component
meshes. Burr does not currently compute exact Boolean overlap volume. It does
not turn an unsupported or inconclusive model into a pass.

See the [roadmap](/burr/roadmap) for the order and explicit non-scope.

## Legacy compatibility

Burr 0.30's `init`, `check`, `explain`, and `stamp` commands remain available
for existing receipt-based workflows. They use `burr-design-data.json` and
declared rulepacks, but they are isolated from `burr .` and are not the basis of
the geometry-native workbench.
