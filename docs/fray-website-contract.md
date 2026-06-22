# Fray Website Burr Gallery Contract

This contract lets the Fray website render Burr gallery content without
regenerating CAD.

## Source

```txt
repo: fraylabs/burr
release_tag: burr-v0.13.2
asset_name: burr-gallery-v0.13.2.zip
asset_url: https://github.com/fraylabs/burr/releases/download/burr-v0.13.2/burr-gallery-v0.13.2.zip
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

## Zip Layout

```txt
burr-gallery-v0.13.2/
  README.md
  manifest.json
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
burr-gallery-v0.13.2/manifest.json
```

Schema:

```json
{
  "schema_version": "burr.gallery-artifact.v1",
  "burr_version": "0.13.2",
  "artifact_id": "burr-gallery-v0.13.2",
  "generated_at": "ISO-8601 timestamp",
  "source": {
    "repository": "fraylabs/burr",
    "tag": "burr-v0.13.2"
  },
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
  "release_tag": "burr-v0.13.2",
  "asset_name": "burr-gallery-v0.13.2.zip"
}
```

This keeps the website independent of whether assets later come from GitHub
release assets, a CDN, or a checked-in `assets/` folder.
