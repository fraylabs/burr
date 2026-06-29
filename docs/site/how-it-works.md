# How Burr Works

Burr is design-rule checking for CAD-as-code.

The basic loop is:

```txt
design.py
  -> generated STEP
  -> burr-design-data.json
  -> burr check
  -> burr-receipt.json
  -> burr explain
```

The CAD source is still normal `build123d`. The difference is that important
mechanical features use Burr helpers instead of anonymous raw cuts.

## Python Source

```python
from build123d import Box, BuildPart, Locations, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole

design = BurrDesignData(
    artifact_id="actuator-mount",
    artifact_type="actuator_mount",
    units="mm",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)

design.source("design.py")
design.artifact("actuator.step")
design.part("housing", bbox_min=(-42, -16, 0), bbox_max=(42, 16, 26))

with BuildPart() as housing:
    with Locations((0, 0, 13)):
        Box(84, 32, 26)

    # This both cuts the CAD hole and records Burr metadata.
    m3_clearance_hole(
        design,
        feature_id="m3_lower_left",
        part="housing",
        center=(39.5, -8, 8),
        axis=(1, 0, 0),
        role="loaded_mount",
    )

export_step(housing.part, "actuator.step")
design.write(DESIGN_DATA_FILE)
```

The helper creates CAD geometry and records design intent into
`burr-design-data.json`.

Counterbore helpers record both the smaller bore and the larger head recess, so
Burr can check STEP presence and edge material around the recess itself.
Bearing-seat helpers record the seat diameter and shoulder, so Burr can check
that the bearing seat exists in STEP and still has enough host material around
the loaded support envelope.

## CLI Loop

```bash
uv run python design.py
burr check .
burr explain .
```

`burr check .` reads `burr-design-data.json`, checks source and artifact
freshness, runs the selected rulepack, and writes `burr-receipt.json`.

`burr explain .` reads the receipt and returns human-readable repair guidance.
Agents can use `burr explain --json .` for structured repair packets.

## Boundary

Burr is not image verification, FEA, or general STEP understanding. It is
unit-test-style design-rule checking over declared CAD intent, with measurable
receipts that humans and agents can use to repair generated CAD.
