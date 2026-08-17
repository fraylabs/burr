# Rulepack Reference

A rulepack selects which declared design features Burr checks.

Rulepacks are JSON files with schema `burr.rulepack.v1`.

The source and future public npm tarball include the machine-readable contract
at `schemas/burr.rulepack.v1.schema.json`.

## Shape

```json
{
  "schema_version": "burr.rulepack.v1",
  "id": "actuator_mount",
  "version": "0.14.0",
  "artifact_type": "actuator_mount",
  "process_kind": "FDM",
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
    },
    {
      "id": "bearing_seat_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "bearing_seat",
        "intent_any": ["mechanical_interface"],
        "role_any": ["loaded_bearing_support", "shaft_support"]
      },
      "diameter_field": "seat_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "m3_standoff_boss_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "standoff_boss",
        "fastener": "M3",
        "intent_any": ["mechanical_interface"]
      },
      "center_field": "boss_center_mm",
      "diameter_field": "boss_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "heat_set_insert_pocket_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "heat_set_insert_pocket",
        "insert": "M3x5.7",
        "intent_any": ["mechanical_interface"]
      },
      "center_field": "pocket_center_mm",
      "diameter_field": "pocket_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "m3_insert_pocket_back_wall_thickness",
      "kind": "blind_pocket_back_wall_thickness",
      "applies_to": {
        "kind": "heat_set_insert_pocket",
        "insert": "M3x5.7",
        "intent_any": ["mechanical_interface"]
      },
      "min_back_wall_thickness_mm": 2.0
    }
  ]
}
```

`feature_edge_distance` defaults to `diameter_mm` for circular envelopes.
Counterbore rules should override `diameter_field` to
`counterbore_diameter_mm` so Burr checks the larger screw-head recess, not only
the smaller bore.
Bearing-seat edge rules should override `diameter_field` to
`seat_diameter_mm` so Burr checks the bearing support envelope, not a smaller
shaft or pilot hole.
Standoff-boss edge rules should override `center_field` and `diameter_field` to
`boss_center_mm` and `boss_diameter_mm` so Burr checks the boss footprint, not
the smaller supported screw hole.
Insert-pocket edge rules should override `center_field` and `diameter_field` to
`pocket_center_mm` and `pocket_diameter_mm` so Burr checks the pocket envelope,
not an unrelated feature center.

## Selection

Rulepack selection is required. Design data can select a bundled rulepack:

```json
{
  "schema_version": "burr.design-data.v1",
  "rulepack": "builtin:actuator_mount"
}
```

Or it can select a rulepack file relative to the design data:

```json
{
  "schema_version": "burr.design-data.v1",
  "rulepack": { "path": "../../../rules/captured_slider.rulepack.json" }
}
```

The CLI can override that choice:

```bash
burr check --rulepack builtin:actuator_mount .
burr check --rulepack rules/printed_plate.rulepack.json .
```

Burr does not fall back to a default rulepack when neither form is present.

## Compatibility

`artifact_type` targets the matching design-data field. A mismatch produces an
`incomplete` receipt because the chosen rulepack was not applicable.

`process_kind` is optional. When present, it targets `process.kind` in the
design data. A missing or different design process produces `incomplete`;
omitting `process_kind` from the rulepack leaves process unrestricted.

## Selectors and Validation

The rulepack contract requires non-empty `id`, `version`, and `artifact_type`
fields plus at least one rule. `process_kind` is optional. Unknown top-level or
rule-specific fields fail validation so a misspelling cannot silently weaken
the check.

The supported `applies_to` selector keys are:

```txt
id  kind  kind_any  fastener  insert  intent  intent_any  role_any
```

`insert` matches the declared feature's `insert` value, so a rule for
`M3x5.7` pockets does not silently apply to other insert types. An unknown
selector key is a rulepack contract failure, not an ignored filter.

Rule ids must be unique. Unsupported rule kinds, duplicate ids, unknown
selectors, and missing required rule parameters fail the receipt. Rules that
define a numeric or count range must include at least one bound. This is
different from an incompatibility or coverage gap: invalid rulepack syntax is
`fail`, while an otherwise valid rulepack that cannot establish coverage is
`incomplete`.

## Supported Rule Kinds

```txt
hole_edge_distance             -> hole center has enough distance to a free edge
feature_edge_distance          -> feature envelope has enough material to a free edge
minimum_wall_thickness         -> hole leaves enough printable wall
fastener_support_wall_thickness -> boss/support leaves enough radial material
blind_pocket_back_wall_thickness -> blind pocket leaves enough material behind its bottom
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
declared features and measurements it selects. A passing receipt means that the
selected rulepack was compatible, at least one rule was evaluated, and evaluated
checks passed with complete declared mechanical-feature coverage. Zero evaluated
rule coverage is `incomplete`; a pass does not certify undeclared geometry or the
whole part.
