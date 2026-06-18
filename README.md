# Burr

Burr is a design-rule linter for CAD-as-code workflows.

It checks generated design data and artifacts for mechanical mistakes before
they become prints, prototypes, or expensive debugging sessions.

```txt
design file -> generated part -> burr-design-data.json -> Burr checks -> receipt
```

Burr is not a constraint solver, FEA engine, or universal CAD brain. It does not
design the part. It checks whether generated CAD violates known mechanical
rules.

## Why

Image review is useful, but not enough. A screenshot can show that something
looks suspicious; it cannot reliably prove exact hole distances, hidden
clearances, source/STEP freshness, or rule-specific pass/fail.

Burr turns design data into measurable receipts:

```txt
M3 loaded mounting hole
measured center-to-edge = 8.0 mm
required center-to-edge = 10.2 mm
result = fail by 2.2 mm
```

## Install

For local development:

```bash
npm install
npm test
```

Run the CLI directly:

```bash
node bin/burr.mjs --version
node bin/burr.mjs check examples/linear-actuator-bad
node bin/burr.mjs check examples/linear-actuator-good
```

## Commands

```bash
burr --version
burr check <folder|burr-design-data.json>...
burr stamp <folder|burr-design-data.json>...
```

`check` finds `burr-design-data.json`, runs freshness checks and rulepack
checks, then writes `burr-receipt.json` beside each design data file.

`stamp` computes `sha256` and `size_bytes` for declared source and generated
artifact files.

## Design Data

A lintable CAD artifact folder contains `burr-design-data.json`.

This file is the language-agnostic contract. It can be emitted by build123d,
CadQuery, OpenSCAD, JavaScript CAD, Rust CAD, Fusion scripts, or any tool that
can write JSON.

```json
{
  "schema_version": "burr.design-data.v1",
  "artifact_id": "linear-actuator-bad",
  "artifact_version": "0.1.0",
  "artifact_type": "actuator_mount",
  "units": "mm",
  "source": {
    "path": "source.py",
    "sha256": "..."
  },
  "artifacts": [
    {
      "kind": "step",
      "path": "actuator.step",
      "sha256": "..."
    }
  ],
  "parts": [
    {
      "id": "housing",
      "bbox_mm": {
        "min": [-42, -16, 0],
        "max": [42, 16, 26]
      }
    }
  ],
  "features": [
    {
      "id": "m3_lower_left",
      "part": "housing",
      "kind": "clearance_hole",
      "fastener": "M3",
      "diameter_mm": 3.4,
      "center_mm": [39.5, -8, 8],
      "axis": [1, 0, 0],
      "role": "loaded_mount"
    }
  ]
}
```

## Rulepacks

The included actuator mount rule checks loaded M3 clearance holes:

```json
{
  "schema_version": "burr.rulepack.v1",
  "id": "actuator_mount",
  "version": "0.1.0",
  "rules": [
    {
      "id": "m3_loaded_hole_edge_distance",
      "kind": "hole_edge_distance",
      "applies_to": {
        "kind": "clearance_hole",
        "fastener": "M3",
        "role_any": ["loaded_mount", "mount", "housing_mount"]
      },
      "min_center_to_edge_diameter_multiple": 3.0
    }
  ]
}
```

## Versioning

Burr has three versioned surfaces:

```txt
Burr package version       -> CLI/library behavior
Design data schema version -> JSON shape Burr can read
Rulepack schema version    -> rule syntax Burr can execute
```

Receipts include all three:

```json
{
  "schema_version": "burr.receipt.v1",
  "burr_version": "0.2.0",
  "artifact_version": "0.1.0",
  "rulepack_version": "0.1.0",
  "compatibility": {
    "design_data_schema_version": "burr.design-data.v1",
    "rulepack_schema_version": "burr.rulepack.v1"
  }
}
```

Unsupported design data or rulepack schemas fail lint instead of silently producing
untrustworthy receipts.

Legacy `fray-cad.json` files with schema `fray.cad.artifact.v1` are still read
for transition, but new integrations should emit `burr-design-data.json`.

## Example Result

Bad actuator:

```json
{
  "status": "fail",
  "reason": "insufficient_edge_distance",
  "measured": {
    "center_to_edge_mm": 8,
    "wall_to_edge_mm": 6.3
  },
  "required": {
    "center_to_edge_mm": 10.2,
    "wall_to_edge_mm": 8.5
  },
  "margin_mm": -2.2
}
```

Fixed actuator:

```json
{
  "status": "pass",
  "measured": {
    "center_to_edge_mm": 12,
    "wall_to_edge_mm": 10.3
  },
  "required": {
    "center_to_edge_mm": 10.2,
    "wall_to_edge_mm": 8.5
  },
  "margin_mm": 1.8
}
```

## Status

Early prototype. Current checks are design-data-based. Future versions may derive
more facts directly from STEP topology.
