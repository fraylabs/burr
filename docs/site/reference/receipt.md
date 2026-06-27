# Receipt Reference

`burr-receipt.json` is the proof Burr writes after `burr check`.

The receipt is the final trust signal. Preview images are visual context only.

## Shape

```json
{
  "schema_version": "burr.receipt.v1",
  "burr_version": "0.21.0",
  "artifact_id": "actuator-mount",
  "artifact_type": "actuator_mount",
  "artifact_version": "0.1.0",
  "status": "fail",
  "rulepack_id": "actuator_mount",
  "rulepack_version": "0.8.0",
  "checks": [
    {
      "rule_id": "actuator_mount:m3_loaded_hole_edge_distance",
      "feature_id": "m3_lower_left",
      "status": "fail",
      "reason": "insufficient_edge_distance",
      "message": "Loaded M3 clearance hole is too close to a free edge.",
      "measured": { "center_to_edge_mm": 8.0 },
      "required": { "min_center_to_edge_mm": 10.2 },
      "margin_mm": -2.2
    }
  ],
  "warnings": [],
  "summary": {
    "checks": 12,
    "failures": 1,
    "warnings": 0,
    "features": {
      "declared": 4,
      "checked": 4,
      "unchecked": 0
    }
  }
}
```

## Check Evidence

Each check should be readable by both a human and an agent:

```txt
Feature: m3_lower_left
Rule: loaded M3 edge distance
Measured: 8.0mm
Required: 10.2mm
Margin: -2.2mm
Fix: move the hole inward or increase the housing size.
```

The important fields are `rule_id`, `feature_id`, `status`, `reason`,
`message`, `measured`, `required`, and `margin_mm`.

## Freshness

Receipts include source and artifact freshness checks. If the source hash or
STEP hash is stale, the receipt should not be trusted as proof of the current
files.
