from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, heat_set_insert_pocket


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "insert-pocket-back-wall.step"

housing_length = 6.7
housing_width = 24.0
housing_height = 16.0

design = BurrDesignData(
    artifact_id="build123d-insert-pocket-back-wall-bad",
    artifact_type="actuator_mount",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "housing",
    bbox_min=(-housing_length / 2, -housing_width / 2, -housing_height / 2),
    bbox_max=(housing_length / 2, housing_width / 2, housing_height / 2),
)

with BuildPart() as part:
    Box(housing_length, housing_width, housing_height)
    heat_set_insert_pocket(
        design,
        feature_id="m3_insert_pocket",
        part="housing",
        center=(0, 0, 0),
        axis=(1, 0, 0),
        role="threaded_mount",
        insert="M3x5.7",
        pocket_diameter_mm=4.6,
        pocket_depth_mm=5.7,
        host_depth_mm=housing_length,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
