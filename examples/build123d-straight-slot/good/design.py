from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, straight_slot


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "slot.step"

housing_length = 70.0
housing_width = 42.0
housing_height = 20.0

design = BurrDesignData(
    artifact_id="build123d-straight-slot-good",
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
    straight_slot(
        design,
        feature_id="motor_adjust_slot",
        part="housing",
        center=(0, -8, 0),
        axis=(1, 0, 0),
        span_axis=(0, 1, 0),
        role="loaded_mount",
        width_mm=4.0,
        length_mm=18.0,
        cut_depth_mm=80.0,
        create_geometry=True,
    )
    straight_slot(
        design,
        feature_id="cosmetic_logo_slot",
        part="housing",
        center=(0, 12, 0),
        axis=(1, 0, 0),
        span_axis=(0, 1, 0),
        role="cosmetic",
        intent="cosmetic",
        width_mm=3.0,
        length_mm=10.0,
        cut_depth_mm=80.0,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
