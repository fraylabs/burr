from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, counterbore, straight_slot


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "slotted-motor-plate.step"

length = 18.0
width = 76.0
height = 26.0

design = BurrDesignData(
    artifact_id="gallery-slotted-motor-plate",
    artifact_type="actuator_mount",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "plate",
    bbox_min=(-length / 2, -width / 2, -height / 2),
    bbox_max=(length / 2, width / 2, height / 2),
)


with BuildPart() as part:
    Box(length, width, height)

    straight_slot(
        design,
        feature_id="motor_tension_slot",
        part="plate",
        center=(0, 0, 0),
        axis=(1, 0, 0),
        span_axis=(0, 1, 0),
        role="adjustable_mount",
        width_mm=5.0,
        length_mm=30.0,
        cut_depth_mm=length + 4,
        create_geometry=True,
    )

    straight_slot(
        design,
        feature_id="cosmetic_alignment_mark",
        part="plate",
        center=(0, 0, 8.0),
        axis=(1, 0, 0),
        span_axis=(0, 1, 0),
        role="engraved_mark",
        intent="cosmetic",
        width_mm=3.0,
        length_mm=14.0,
        cut_depth_mm=length + 4,
        create_geometry=True,
    )

    for y in (-26.0, 26.0):
        counterbore(
            design,
            feature_id=f"m3_socket_mount_{'left' if y < 0 else 'right'}",
            part="plate",
            center=(0, y, 0),
            axis=(1, 0, 0),
            role="housing_mount",
            bore_diameter_mm=3.4,
            counterbore_diameter_mm=6.4,
            counterbore_depth_mm=3.0,
            through_depth_mm=length,
            create_geometry=True,
        )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
