from pathlib import Path

from build123d import Box, BuildPart, Locations, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "actuator-housing.step"

housing_length = 72.0
housing_width = 34.0
housing_height = 18.0
m3_diameter = 3.4
hole_z = housing_height / 2.0
mount_holes = {
    "m3_front_left": (-28.0, -8.0, hole_z),
    "m3_front_right": (28.0, -8.0, hole_z),
    "m3_rear_left": (-28.0, 8.0, hole_z),
    "m3_rear_right": (28.0, 8.0, hole_z),
}

design = BurrDesignData(
    artifact_id="build123d-actuator-housing-repair-bad",
    artifact_type="actuator_mount",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "housing",
    bbox_min=(-housing_length / 2.0, -housing_width / 2.0, 0.0),
    bbox_max=(housing_length / 2.0, housing_width / 2.0, housing_height),
)

with BuildPart() as housing:
    with Locations((0, 0, housing_height / 2.0)):
        Box(housing_length, housing_width, housing_height)

    for feature_id, center in mount_holes.items():
        m3_clearance_hole(
            design,
            feature_id=feature_id,
            part="housing",
            center=center,
            axis=(0, 0, 1),
            role="housing_mount",
            diameter_mm=m3_diameter,
            cut_depth_mm=housing_height + 4.0,
        )

export_step(housing.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)

print(f"wrote {BASE_DIR / DESIGN_DATA_FILE}")
