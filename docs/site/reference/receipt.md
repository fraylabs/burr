# Receipt Reference

`burr-receipt.json` is the scoped trust result Burr writes after `burr check`.

The receipt records the selected rulepack, warnings, coverage, and checked
claims. Preview images are visual context only.

The source and future public npm tarball include the machine-readable contract
at `schemas/burr.receipt.v2.schema.json`.

## Shape

The `checks` array below is abbreviated to one representative failure. The
summary and scope counts describe the complete receipt from which it was taken.

```json
{
  "schema_version": "burr.receipt.v2",
  "burr_version": "0.30.0",
  "artifact_id": "actuator-mount",
  "artifact_type": "actuator_mount",
  "artifact_version": "0.1.0",
  "outcome": "fail",
  "status": "fail",
  "rulepack_id": "actuator_mount",
  "rulepack_version": "0.14.0",
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
  "scope": {
    "artifact_type": {
      "design": "actuator_mount",
      "rulepack": "actuator_mount",
      "compatible": true
    },
    "process_kind": {
      "design": "FDM",
      "rulepack": "FDM",
      "restricted": true,
      "compatible": true
    },
    "rules": {
      "declared": 17,
      "evaluated": 6
    },
    "mechanical_features": {
      "declared": 4,
      "checked": 4,
      "unchecked": 0,
      "unchecked_feature_ids": []
    }
  },
  "summary": {
    "checks": 12,
    "failures": 1,
    "incomplete_checks": 0,
    "warnings": 0,
    "rules": {
      "declared": 17,
      "evaluated": 6
    },
    "features": {
      "declared": 4,
      "checked": 4,
      "unchecked": 0,
      "mechanical_declared": 4,
      "mechanical_checked": 4,
      "mechanical_unchecked": 0
    }
  }
}
```

`outcome` is the canonical three-state result. Within receipt v2, `status`
mirrors it for callers that still use the older field name; this does not make
the v2 outcome set backward-compatible with v1.

Receipt v2 introduces the `incomplete` state and the required scope fields. A
consumer written for `burr.receipt.v1` must opt into v2 and handle all three
outcomes; it must never interpret an unknown schema or outcome as `pass`.

## Outcomes

| Outcome | Meaning |
| --- | --- |
| `pass` | The selected rulepack was compatible, at least one rule was evaluated, evaluated checks passed, and every declared mechanical-interface feature received coverage. |
| `incomplete` | Burr ran but could not establish the pass claim because compatibility, rule coverage, mechanical-feature coverage, or pair-spacing evidence was incomplete. |
| `fail` | A checked claim failed, metadata is stale/invalid, or the rulepack contract is invalid. |

Examples of `incomplete` include artifact or process incompatibility, zero
evaluated rule coverage, an unchecked mechanical-interface feature, and a
pair-spacing rule with fewer than two candidates. Warnings and coverage remain
visible in the receipt and terminal output. Explicitly non-mechanical unchecked
features are still reported but do not block `pass`.

## Scope and Warnings

`scope.artifact_type` and `scope.process_kind` show whether the selected
rulepack targets the design. `scope.rules` reports declared and evaluated rule
counts. `scope.mechanical_features` reports required mechanical coverage and
names any unchecked feature ids. The same feature detail remains available in
`summary.features`, alongside coverage for explicitly non-mechanical features.

Warnings carry `affects_outcome`. A warning with `true` blocks a pass and
explains an `incomplete` result unless a failed check takes precedence. A
warning with `false`, such as an optional rule with no applicable features,
remains visible without independently blocking a pass.

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

## Trust Boundary

A passing receipt proves only the declared claims selected by the named
rulepack, using the recorded metadata, freshness checks, and any stated STEP
evidence. It does not prove undeclared geometry, infer every mechanical feature,
perform FEA, or certify manufacturing fitness. An `incomplete` receipt is not a
weaker pass and must not be presented as verified.

## Freshness

Receipts include source and artifact freshness checks. If the source hash or
STEP hash is stale, the receipt should not be trusted as proof of the current
files.
