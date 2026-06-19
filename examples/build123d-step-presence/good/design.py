from pathlib import Path

from build123d import Box, BuildPart, Locations, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "presence.step"

housing_length = 40.0
housing_width = 20.0
housing_height = 16.0
m3_diameter = 3.4

design = BurrDesignData(
    artifact_id="build123d-step-presence-good",
    artifact_type="actuator_mount",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "housing",
    bbox_min=(-housing_length / 2.0, -housing_width / 2.0, 0),
    bbox_max=(housing_length / 2.0, housing_width / 2.0, housing_height),
)

with BuildPart() as housing:
    with Locations((0, 0, housing_height / 2.0)):
        Box(housing_length, housing_width, housing_height)

    m3_clearance_hole(
        design,
        feature_id="m3_claimed",
        part="housing",
        center=(0, 0, housing_height / 2.0),
        axis=(1, 0, 0),
        role="alignment",
        diameter_mm=m3_diameter,
        cut_depth_mm=8.0,
    )

export_step(housing.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)

print(f"wrote {BASE_DIR / DESIGN_DATA_FILE}")
