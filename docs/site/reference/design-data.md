# Design Data Reference

`burr-design-data.json` is the language-agnostic contract Burr checks.

It can be emitted by `burr-build123d`, CadQuery, OpenSCAD, JavaScript CAD, Rust
CAD, Fusion scripts, or any tool that can write JSON.

## Minimal Shape

```json
{
  "schema_version": "burr.design-data.v1",
  "artifact_id": "actuator-mount",
  "artifact_version": "0.1.0",
  "artifact_type": "actuator_mount",
  "units": "mm",
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
| `source` | Source file path and hash used for freshness checks. |
| `artifacts` | Generated outputs, usually STEP, with hashes. |
| `parts` | Declared part envelopes or named bodies. |
| `features` | Declared design intent Burr can check. |
| `rulepack` | Optional rulepack path selected by the design data. |
| `measurements` | Optional named measurements for custom rulepacks. |

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
```
