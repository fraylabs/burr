# Design Data Reference

`burr-design-data.json` is the language-agnostic contract Burr checks.

It can be emitted by `burr-build123d`, CadQuery, OpenSCAD, JavaScript CAD, Rust
CAD, Fusion scripts, or any tool that can write JSON.

The source and future public npm tarball include the machine-readable contract
at `schemas/burr.design-data.v1.schema.json`.

## Minimal Shape

```json
{
  "schema_version": "burr.design-data.v1",
  "artifact_id": "actuator-mount",
  "artifact_version": "0.1.0",
  "artifact_type": "actuator_mount",
  "units": "mm",
  "process": {
    "kind": "FDM",
    "material": "PETG",
    "nozzle_mm": 0.4
  },
  "rulepack": "builtin:actuator_mount",
  "source": {
    "path": "design.py",
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
      "intent": "mechanical_interface",
      "fastener": "M3",
      "diameter_mm": 3.4,
      "center_mm": [39.5, -8, 8],
      "axis": [1, 0, 0],
      "role": "loaded_mount"
    }
  ]
}
```

## Top-Level Fields

| Field | Purpose |
| --- | --- |
| `schema_version` | Must be `burr.design-data.v1` for this Burr release. |
| `artifact_id` | Stable id for the generated CAD artifact. |
| `artifact_version` | Optional design version. |
| `artifact_type` | Selects rulepack compatibility, such as `actuator_mount`. |
| `units` | Must be `mm`. |
| `process` | Optional manufacturing context. `process.kind` is checked when the selected rulepack declares `process_kind`. |
| `source` | Source file path and hash used for freshness checks. |
| `artifacts` | Generated outputs, usually STEP, with hashes. |
| `parts` | Declared part envelopes or named bodies. |
| `features` | Declared design intent Burr can check. |
| `rulepack` | Required unless the CLI receives `--rulepack`; accepts a built-in selector or rulepack file reference. |
| `measurements` | Optional named measurements for custom rulepacks. |

## Explicit Rulepack Selection

Burr never silently chooses a default rulepack. Select the bundled actuator
rulepack directly:

```json
{ "rulepack": "builtin:actuator_mount" }
```

Or select a file relative to `burr-design-data.json`:

```json
{ "rulepack": { "path": "rules/my-part.rulepack.json" } }
```

`burr check --rulepack <selector>` overrides the design-data selection. Missing
selection is an invocation/configuration error rather than a receipt-backed
result.

## Feature Intent

Burr does not infer that every cylinder or hole in a STEP file is mechanically
important. A STEP file may contain vents, lightening holes, cable routes,
cosmetic cuts, construction reliefs, bosses, fillets, and unrelated round faces.

Use `intent` to separate mechanical interfaces from incidental geometry:

```txt
mechanical_interface  -> judged by mechanical rulepacks
weight_reduction      -> declared if useful, but not judged by mount rules
fluid_or_air_path     -> separate rules, not screw-mount rules
manufacturing_feature -> process-specific rules only
cosmetic              -> normally unjudged
reference             -> linkage/evidence context, not an independent claim
```

Missing `intent` is treated as `mechanical_interface` for compatibility.
Every declared mechanical-interface feature must be selected by at least one
evaluated rule for a passing receipt. An unchecked mechanical feature makes the
outcome `incomplete`; explicitly non-mechanical unchecked features are reported
but do not block a pass.

Only these documented non-mechanical values are coverage-exempt. Burr treats
an unknown or misspelled intent as coverage-required, preserving support for
custom rule selectors without allowing a typo to bypass mechanical coverage.

## Standoff Boss Links

For `kind: "standoff_boss"`, `supports_feature_id` links the raised boss to the
declared `clearance_hole` or `heat_set_insert_pocket` it supports.

The linked feature should expose `center_mm`, `axis`, and a comparable diameter
such as `support_diameter_mm`, `diameter_mm`, or `pocket_diameter_mm`. Burr uses
those fields to check that the boss is centered on the supported feature instead
of merely existing somewhere nearby.

For `kind: "counterbore"`, `part` should reference a part with `bbox_mm`,
`center_mm` and `axis` locate the through-hole, and `counterbore_diameter_mm`
describes the larger screw-head recess. Edge-material rules use that larger
diameter so a counterbore can fail even when the smaller bore would leave enough
material.

For `kind: "heat_set_insert_pocket"`, `part` should reference a part with
`bbox_mm`, `pocket_center_mm` locates the blind pocket cylinder, and
`bottom_center_mm` locates the pocket bottom. Back-wall rules measure from that
bottom point to the host part bbox face in the pocket-bottom direction.
Edge-material rules use `pocket_center_mm` and `pocket_diameter_mm` so an insert
pocket can fail when the full pocket is too close to a free edge, even if the
threaded feature exists.

For `kind: "standoff_boss"`, `part` should reference a part with `bbox_mm`,
`boss_center_mm` locates the raised boss footprint, and `boss_diameter_mm`
describes the boss envelope. Edge-material rules use those fields so a boss can
fail when the support cylinder is too close to a free edge, even if the supported
hole is inset enough.

For `kind: "bearing_seat"`, `part` should reference a part with `bbox_mm`,
`center_mm` and `axis` locate the seat, and `seat_diameter_mm` describes the
bearing support envelope. Edge-material rules use that diameter so a bearing
seat can fail when the seat is too close to a free edge even if the STEP seat
geometry exists.
