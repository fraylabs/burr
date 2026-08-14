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
design.rulepack("builtin:actuator_mount")
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
Rulepack selection is explicit: design data can name a built-in selector such as
`builtin:actuator_mount` or a rulepack file, and `--rulepack` can select one at
the command line. Burr does not silently choose a default rulepack.

The terminal output includes warnings and checked/unchecked feature coverage.
The receipt outcome is `pass`, `incomplete`, or `fail`:

- `pass` means the selected rulepack was compatible, evaluated checks passed,
  and all declared mechanical-interface features received coverage;
- `incomplete` means Burr could not establish that trust claim, such as when the
  rulepack is incompatible or mechanical coverage is missing;
- `fail` means a checked claim failed or the rulepack contract is invalid.

Explicitly non-mechanical unchecked features do not block a pass.

`burr explain .` reads the receipt and returns human-readable repair guidance.
Agents can use `burr explain --json .` for structured repair packets.

## Boundary

Burr is not image verification, FEA, a constraint solver, or general STEP
understanding. It is unit-test-style design-rule checking over declared CAD
intent. A pass applies only to the selected rulepack, declared features, and
evidence named in the receipt; it does not certify the whole part or find every
possible mechanical defect.
