# How Burr Works

Burr opens ordinary CAD files locally:

```txt
burr .
  -> discover STEP, STL, and GLB files
  -> preserve their folder hierarchy
  -> compile the selected model with Look
  -> render it in a local browser
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

## Geometry-native direction

Burr's next check will inspect actual STEP assembly geometry for overlapping
solid volume and connect each interference finding back to the viewer. Later
checks should reuse those stable body, face, and measurement references.

See the [roadmap](/burr/roadmap) for the order and explicit non-scope.

## Legacy compatibility

Burr 0.30's `init`, `check`, `explain`, and `stamp` commands remain available
for existing receipt-based workflows. They use `burr-design-data.json` and
declared rulepacks, but they are isolated from `burr .` and are not the basis of
the geometry-native workbench.
