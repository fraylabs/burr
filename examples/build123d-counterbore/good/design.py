from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, counterbore


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "counterbore.step"

housing_length = 20.0
housing_width = 32.0
housing_height = 18.0

design = BurrDesignData(
    artifact_id="build123d-counterbore-good",
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
        feature_id="m3_mount_counterbore",
        part="housing",
        center=(0, -6, 0),
        axis=(1, 0, 0),
        role="loaded_mount",
        bore_diameter_mm=3.4,
        counterbore_diameter_mm=6.8,
        counterbore_depth_mm=4.0,
        through_depth_mm=housing_length,
        create_geometry=True,
    )
    counterbore(
        design,
        feature_id="cosmetic_counterbore",
        part="housing",
        center=(0, 8, 0),
        axis=(1, 0, 0),
        role="cosmetic",
        intent="cosmetic",
        bore_diameter_mm=3.0,
        counterbore_diameter_mm=5.8,
        counterbore_depth_mm=3.0,
        through_depth_mm=housing_length,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
