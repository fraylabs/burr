from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, bearing_seat


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "bearing-seat.step"

housing_length = 24.0
housing_width = 36.0
housing_height = 30.0

design = BurrDesignData(
    artifact_id="build123d-bearing-seat-good",
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
    bearing_seat(
        design,
        feature_id="bearing_608_seat",
        part="housing",
        center=(0, -6, 0),
        axis=(1, 0, 0),
        role="bearing_support",
        bearing="608",
        seat_diameter_mm=22.0,
        seat_depth_mm=7.0,
        host_depth_mm=housing_length,
        create_geometry=True,
    )
    bearing_seat(
        design,
        feature_id="cosmetic_bearing_recess",
        part="housing",
        center=(0, 10, 0),
        axis=(1, 0, 0),
        role="cosmetic",
        intent="cosmetic",
        bearing="decorative",
        seat_diameter_mm=10.0,
        seat_depth_mm=3.0,
        host_depth_mm=housing_length,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
