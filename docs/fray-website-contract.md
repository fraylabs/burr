# Fray Website Burr Gallery Contract

This contract lets the Fray website render Burr gallery content without
regenerating CAD.

## Source

```txt
repo: fraylabs/burr
release_tag: burr-v0.16.0
asset_name: burr-gallery-v0.16.0.zip
asset_url: https://github.com/fraylabs/burr/releases/download/burr-v0.16.0/burr-gallery-v0.16.0.zip
```

The website should treat Burr release assets as read-only product data.

## Rules

- Burr owns CAD source, generated gallery artifacts, receipts, and manifest
  shape.
- Fray website consumes the zip and renders it.
- Fray website must not regenerate CAD.
- PNG previews are visual review artifacts only.
- Burr receipts are the proof artifacts.
- A passing gallery card should be based on receipt `status: "pass"`, not on
  image appearance.
- A failing gallery card is allowed when `expectation: "fail"` and the receipt
  has at least one failed check. These are intentional negative fixtures.
- A before/after repair proof is a pair of receipt-backed states: the before
  card fails for a declared mechanical reason, and the after card passes after
  the CAD is repaired.

## Zip Layout

```txt
burr-gallery-v0.16.0/
  README.md
  manifest.json
  repair-reports/
    actuator-housing-edge-distance.json
    actuator-housing-edge-distance.md
  shaft-bearing-bracket/
    shaft-bearing-bracket.png
    shaft-bearing-bracket.receipt.json
    shaft-bearing-bracket.design-data.json
  slotted-motor-plate/
    slotted-motor-plate.png
    slotted-motor-plate.receipt.json
    slotted-motor-plate.design-data.json
  electronics-standoff-deck/
    electronics-standoff-deck.png
    electronics-standoff-deck.receipt.json
    electronics-standoff-deck.design-data.json
  dense-random-hole-plate/
    dense-random-hole-plate.png
    dense-random-hole-plate.receipt.json
    dense-random-hole-plate.design-data.json
  bad-bearing-seat-missing-shoulder/
    bad-bearing-seat-missing-shoulder.png
    bad-bearing-seat-missing-shoulder.receipt.json
    bad-bearing-seat-missing-shoulder.design-data.json
  bad-counterbore-missing-recess/
    bad-counterbore-missing-recess.png
    bad-counterbore-missing-recess.receipt.json
    bad-counterbore-missing-recess.design-data.json
  bad-insert-pocket-through-hole/
    bad-insert-pocket-through-hole.png
    bad-insert-pocket-through-hole.receipt.json
    bad-insert-pocket-through-hole.design-data.json
  bad-slot-disconnected-holes/
    bad-slot-disconnected-holes.png
    bad-slot-disconnected-holes.receipt.json
    bad-slot-disconnected-holes.design-data.json
```

## Manifest

Manifest path:

```txt
burr-gallery-v0.16.0/manifest.json
```

Schema:

```json
{
  "schema_version": "burr.gallery-artifact.v1",
  "burr_version": "0.16.0",
  "artifact_id": "burr-gallery-v0.16.0",
  "generated_at": "ISO-8601 timestamp",
  "source": {
    "repository": "fraylabs/burr",
    "tag": "burr-v0.16.0"
  },
  "repair_reports": [
    {
      "id": "actuator-housing-edge-distance",
      "title": "Actuator Housing Edge-Distance Repair",
      "kind": "before_after",
      "before_slug": "bad-actuator-housing-edge-distance",
      "after_slug": "fixed-actuator-housing",
      "status": "pass",
      "report_json": "repair-reports/actuator-housing-edge-distance.json",
      "report_markdown": "repair-reports/actuator-housing-edge-distance.md"
    }
  ],
  "examples": [
    {
      "slug": "shaft-bearing-bracket",
      "title": "Shaft Bearing Bracket",
      "expectation": "pass",
      "group": "functional-good",
      "preview": "shaft-bearing-bracket/shaft-bearing-bracket.png",
      "receipt": "shaft-bearing-bracket/shaft-bearing-bracket.receipt.json",
      "design_data": "shaft-bearing-bracket/shaft-bearing-bracket.design-data.json",
      "status": "pass",
      "failed_rules": [],
      "checked_features": ["bearing_608_primary"],
      "unchecked_features": ["cosmetic_relief_recess"]
    },
    {
      "slug": "bad-counterbore-missing-recess",
      "title": "Bad Counterbore Missing Recess",
      "expectation": "fail",
      "group": "mistake-caught",
      "preview": "bad-counterbore-missing-recess/bad-counterbore-missing-recess.png",
      "receipt": "bad-counterbore-missing-recess/bad-counterbore-missing-recess.receipt.json",
      "design_data": "bad-counterbore-missing-recess/bad-counterbore-missing-recess.design-data.json",
      "status": "fail",
      "failed_rules": [
        {
          "rule_id": "actuator_mount:counterbore_step_presence",
          "feature_id": "m3_mount_counterbore",
          "reason": "missing_declared_feature",
          "message": "Declared counterbore m3_mount_counterbore is missing from the STEP artifact."
        }
      ],
      "checked_features": ["m3_mount_counterbore"],
      "unchecked_features": ["cosmetic_counterbore"]
    }
  ]
}
```

`checked_features` and `unchecked_features` are display summaries copied from
the Burr receipt. The receipt remains the source of proof if the website needs
more detail. `failed_rules` is a display summary copied from failed receipt
checks for intentional negative fixtures.

## Burr 0.14 Repair Narrative

For Burr 0.14, render the actuator repair proof as one simple loop:

```txt
bad CAD -> Burr check -> explain fix order -> fixed CAD passes
```

Use the existing manifest and receipt fields. No website-side CAD regeneration
or new geometry interpretation is required.

Recommended repair copy:

```txt
Before: Burr caught the declared actuator mistake.
Check: the receipt records the measured failure.
Fix order: burr explain says what to repair first.
After: the repaired actuator has a passing Burr receipt.
```

For the bad actuator card, use the failed receipt summary to say what Burr
caught. For the fixed actuator card, use `status: "pass"` to say the declared
actuator checks now pass. Do not imply that Burr designed the repair; Burr
checked the before state, explained the fix order, and checked the after state.

## Burr 0.16 Repair Reports

Burr 0.16 repair reports include portable `repair_actions[]` in the same gallery
artifact. A report is not a new verifier. It is a receipt-backed summary of the
before/after loop, with suggested actions derived from the bad/fixed receipts and
design data only.

Report JSON schema:

```json
{
  "schema_version": "burr.repair-report.v1",
  "id": "actuator-housing-edge-distance",
  "title": "Actuator Housing Edge-Distance Repair",
  "status": "pass",
  "loop": "bad CAD -> Burr check -> explain fix order -> fixed CAD passes",
  "first_fix": "Move the loaded M3 holes inward or increase the surrounding housing size.",
  "before": {
    "slug": "bad-actuator-housing-edge-distance",
    "status": "fail",
    "receipt": "bad-actuator-housing-edge-distance/bad-actuator-housing-edge-distance.receipt.json",
    "failures": 4
  },
  "repair_actions": [
    {
      "feature_id": "m3_front_left",
      "action": "move_feature",
      "parameter": "center_mm",
      "before_value_mm": [-28, -8, 9],
      "after_value_mm": [-22, -12, 9],
      "suggested_delta_mm": [6, -4, 0],
      "failure_reason": "insufficient_edge_distance",
      "reason": "Move m3_front_left from [-28, -8, 9] mm to [-22, -12, 9] mm so center-to-edge increases from 8 mm to at least 10.2 mm.",
      "measured": { "center_to_edge_mm": 8.0 },
      "required": { "center_to_edge_mm": 10.2 },
      "margin_mm": -2.2,
      "verifies_against_after_feature": {
        "feature_id": "m3_front_left",
        "status": "pass",
        "margin_mm": 1.8
      }
    }
  ],
  "after": {
    "slug": "fixed-actuator-housing",
    "status": "pass",
    "receipt": "fixed-actuator-housing/fixed-actuator-housing.receipt.json"
  }
}
```

The website should render report Markdown or selected report JSON fields only
when `manifest.repair_reports[]` declares the files. Do not infer report paths.

## Burr 0.16 Repair Actions

Burr 0.16 adds `repair_actions[]` to the repair report JSON. These are
machine-readable suggestions derived from the bad/fixed receipts and design
data. They are not automatic CAD edits.

Example action:

```json
{
  "feature_id": "m3_front_left",
  "action": "move_feature",
  "parameter": "center_mm",
  "before_value_mm": [-28, -8, 9],
  "after_value_mm": [-22, -12, 9],
  "suggested_delta_mm": [6, -4, 0],
  "failure_reason": "insufficient_edge_distance",
  "reason": "Move m3_front_left from [-28, -8, 9] mm to [-22, -12, 9] mm so center-to-edge increases from 8 mm to at least 10.2 mm.",
  "measured": { "center_to_edge_mm": 8.0 },
  "required": { "center_to_edge_mm": 10.2 },
  "margin_mm": -2.2,
  "verifies_against_after_feature": {
    "feature_id": "m3_front_left",
    "status": "pass",
    "margin_mm": 1.8
  }
}
```

## Website Rendering

For each `examples[]` entry, render:

- `title`
- PNG at `preview`
- receipt `status`
- `expectation`
- checked feature count and names
- unchecked feature count and names
- failed rule summaries when `status: "fail"`
- optional link/download to the full receipt JSON

Recommended card copy:

```txt
Status: pass
Proof: Burr receipt
Visual: generated STEP preview
```

For an intentional negative fixture:

```txt
Status: fail
Expectation: fail
Proof: Burr caught the declared mistake
Visual: generated STEP preview
```

Do not call a preview "verified" by itself. Call the receipt verified.

## Update Policy

For a new Burr release:

1. Run `npm run check:gallery:artifact` in the Burr repo.
2. Upload `artifacts/releases/burr-gallery-v<version>.zip` to the matching
   GitHub release tag.
3. Update the website config to the new `release_tag`, `asset_name`, and
   `asset_url`.

The website data model should use:

```json
{
  "repo": "fraylabs/burr",
  "release_tag": "burr-v0.16.0",
  "asset_name": "burr-gallery-v0.16.0.zip"
}
```

This keeps the website independent of whether assets later come from GitHub
release assets, a CDN, or a checked-in `assets/` folder.
