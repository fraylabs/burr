# Rulepack Reference

A rulepack selects which declared design features Burr checks.

Rulepacks are JSON files with schema `burr.rulepack.v1`.

## Shape

```json
{
  "schema_version": "burr.rulepack.v1",
  "id": "actuator_mount",
  "version": "0.11.0",
  "artifact_type": "actuator_mount",
  "rules": [
    {
      "id": "m3_loaded_hole_edge_distance",
      "kind": "hole_edge_distance",
      "applies_to": {
        "kind": "clearance_hole",
        "fastener": "M3",
        "role_any": ["loaded_mount"]
      },
      "min_center_to_edge_diameter_multiple": 3.0
    },
    {
      "id": "mechanical_slot_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "straight_slot",
        "intent_any": ["mechanical_interface"]
      },
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "counterbore_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "counterbore",
        "intent_any": ["mechanical_interface"]
      },
      "diameter_field": "counterbore_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    }
  ]
}
```

`feature_edge_distance` defaults to `diameter_mm` for circular envelopes.
Counterbore rules should override `diameter_field` to
`counterbore_diameter_mm` so Burr checks the larger screw-head recess, not only
the smaller bore.

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

## Supported Rule Kinds

```txt
hole_edge_distance             -> hole center has enough distance to a free edge
feature_edge_distance          -> feature envelope has enough material to a free edge
minimum_wall_thickness         -> hole leaves enough printable wall
fastener_support_wall_thickness -> boss/support leaves enough radial material
standoff_boss_support_link     -> boss references and aligns with the hole or insert it supports
feature_presence               -> declared feature exists in the exported STEP
feature_count                  -> declared feature inventory count is in range
feature_pair_spacing           -> declared feature pair leaves enough ligament
numeric_range                  -> declared measurement is in range
```

`standoff_boss_support_link` checks metadata relationship, not STEP geometry:

```json
{
  "id": "m3_standoff_boss_support_link",
  "kind": "standoff_boss_support_link",
  "applies_to": {
    "kind": "standoff_boss",
    "fastener": "M3",
    "intent_any": ["mechanical_interface"]
  },
  "centerline_tolerance_mm": 0.25,
  "support_diameter_tolerance_mm": 0.05,
  "axis_dot_min": 0.99
}
```

## Boundary

Rulepacks are design-rule checks, not constraint solvers. A rulepack only checks
declared features and measurements it selects.
