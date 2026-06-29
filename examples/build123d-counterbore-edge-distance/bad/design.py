from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, counterbore


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "counterbore-edge.step"

housing_length = 20.0
housing_width = 32.0
housing_height = 24.0

design = BurrDesignData(
    artifact_id="build123d-counterbore-edge-distance-bad",
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
    counterbore(
        design,
        feature_id="m3_head_recess",
        part="housing",
        center=(0, -11, 0),
        axis=(1, 0, 0),
        role="loaded_mount",
        bore_diameter_mm=3.4,
        counterbore_diameter_mm=6.8,
        counterbore_depth_mm=4.0,
        through_depth_mm=housing_length,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
