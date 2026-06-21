# Fray Website Burr Gallery Contract

This contract lets the Fray website render Burr gallery content without
regenerating CAD.

## Source

```txt
repo: fraylabs/burr
release_tag: burr-v0.10.0
asset_name: burr-gallery-v0.10.0.zip
asset_url: https://github.com/fraylabs/burr/releases/download/burr-v0.10.0/burr-gallery-v0.10.0.zip
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

## Zip Layout

```txt
burr-gallery-v0.10.0/
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
```

## Manifest

Manifest path:

```txt
burr-gallery-v0.10.0/manifest.json
```

Schema:

```json
{
  "schema_version": "burr.gallery-artifact.v1",
  "burr_version": "0.10.0",
  "artifact_id": "burr-gallery-v0.10.0",
  "generated_at": "ISO-8601 timestamp",
  "source": {
    "repository": "fraylabs/burr",
    "tag": "burr-v0.10.0"
  },
  "examples": [
    {
      "slug": "shaft-bearing-bracket",
      "title": "Shaft Bearing Bracket",
      "preview": "shaft-bearing-bracket/shaft-bearing-bracket.png",
      "receipt": "shaft-bearing-bracket/shaft-bearing-bracket.receipt.json",
      "design_data": "shaft-bearing-bracket/shaft-bearing-bracket.design-data.json",
      "status": "pass",
      "checked_features": ["bearing_608_primary"],
      "unchecked_features": ["cosmetic_relief_recess"]
    }
  ]
}
```

`checked_features` and `unchecked_features` are display summaries copied from
the Burr receipt. The receipt remains the source of proof if the website needs
more detail.

## Website Rendering

For each `examples[]` entry, render:

- `title`
- PNG at `preview`
- receipt `status`
- checked feature count and names
- unchecked feature count and names
- optional link/download to the full receipt JSON

Recommended card copy:

```txt
Status: pass
Proof: Burr receipt
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
  "release_tag": "burr-v0.10.0",
  "asset_name": "burr-gallery-v0.10.0.zip"
}
```

This keeps the website independent of whether assets later come from GitHub
release assets, a CDN, or a checked-in `assets/` folder.

