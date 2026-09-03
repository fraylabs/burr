# CLI Reference

Burr ships as one Rust CLI:

```txt
burr <folder>
burr --help
burr --version
```

## `burr <folder>`

Open a local browser for the STEP, STL, and GLB files below `folder`:

```bash
burr .
burr models
```

The sidebar preserves nested folders while hiding unrelated files and common
build directories. Burr renders with Look on the local machine, reports its
current loading stage, reuses content-matched viewers across restarts, and
refreshes the active model when its source changes.

The viewport starts in X-ray mode so enclosed or occluded component occurrences
remain visible. Solid mode is available from the viewport switch.

For STEP assemblies with at least two component occurrences, opening the Checks
tab runs Burr's geometry-native `assembly-interference` check. Face-touching pairs
are accepted; surface crossings, containment, and coincident occurrences fail.
Unsupported or inconclusive inputs report `incomplete`, not `pass`.

The selected model's versioned result is available locally at:

```txt
GET /api/checks?path=<project-relative-model-path>
```

When the nearest project root contains `.burr/config.toml`, Burr scans only the
declared `project.models` directories. Missing configuration retains the
zero-configuration folder behavior. Invalid configuration stops startup with
an explicit error.
