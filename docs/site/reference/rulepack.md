# Rulepack Reference

A rulepack selects which declared design features Burr checks.

Rulepacks are JSON files with schema `burr.rulepack.v1`.

## Shape

```json
{
  "schema_version": "burr.rulepack.v1",
  "id": "actuator_mount",
  "version": "0.8.0",
  "artifact_type": "actuator_mount",
  "rules": [
    {
      "id": "m3_loaded_hole_edge_distance",
      "kind": "hole_edge_distance",
      "feature_kind": "clearance_hole",
      "fastener": "M3",
      "role": "loaded_mount",
      "min_center_to_edge_mm": 10.2
    }
  ]
}
```

## Selection

Design data can choose a rulepack:

```json
{
  "schema_version": "burr.design-data.v1",
  "rulepack": { "path": "../../../rules/captured_slider.rulepack.json" }
}
```

The CLI can override that choice:

```bash
burr check --rulepack rules/printed_plate.rulepack.json .
```

## Boundary

Rulepacks are design-rule checks, not constraint solvers. A rulepack only checks
declared features and measurements it selects.
