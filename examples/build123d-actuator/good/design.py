from pathlib import Path

from build123d import Box, BuildPart, Locations, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "actuator.step"

housing_length = 86.0
housing_width = 48.0
housing_height = 40.0
m3_hole_y = 12.0
m3_hole_z = 12.0
m3_diameter = 3.4

design = BurrDesignData(
    artifact_id="build123d-actuator-good",
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
        feature_id="m3_lower_left",
        part="housing",
        center=(housing_length / 2.0 - 3.0, -m3_hole_y, m3_hole_z),
        axis=(1, 0, 0),
        role="loaded_mount",
        diameter_mm=m3_diameter,
        cut_depth_mm=8.0,
    )

export_step(housing.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)

print(f"wrote {BASE_DIR / DESIGN_DATA_FILE}")
