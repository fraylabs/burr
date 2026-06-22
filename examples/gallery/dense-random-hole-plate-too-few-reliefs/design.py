from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole, straight_slot


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "dense-random-hole-plate-too-few-reliefs.step"

length = 16.0
width = 96.0
height = 62.0

design = BurrDesignData(
    artifact_id="gallery-dense-random-hole-plate-too-few-reliefs",
    artifact_type="printed_plate",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/printed_plate.rulepack.json")
design.part(
    "plate",
    bbox_min=(-length / 2, -width / 2, -height / 2),
    bbox_max=(length / 2, width / 2, height / 2),
)

cosmetic_holes = [
    ("cosmetic_grid_a1", -36.0, -20.0, 2.6),
    ("cosmetic_grid_a2", -24.0, -20.0, 2.6),
    ("cosmetic_grid_a3", -12.0, -20.0, 2.6),
    ("cosmetic_grid_a4", 12.0, -20.0, 2.6),
    ("cosmetic_grid_a5", 24.0, -20.0, 2.6),
    ("cosmetic_grid_a6", 36.0, -20.0, 2.6),
    ("cosmetic_grid_b1", -30.0, 5.0, 3.2),
]

with BuildPart() as part:
    Box(length, width, height)

    for y in (-32.0, 32.0):
        m3_clearance_hole(
            design,
            feature_id=f"m3_loaded_mount_{'left' if y < 0 else 'right'}",
            part="plate",
            center=(0, y, 16.0),
            axis=(1, 0, 0),
            role="loaded_mount",
            diameter_mm=3.4,
            cut_depth_mm=length + 4,
            create_geometry=True,
        )

    straight_slot(
        design,
        feature_id="adjustable_sensor_slot",
        part="plate",
        center=(0, 0, -15.0),
        axis=(1, 0, 0),
        span_axis=(0, 1, 0),
        role="adjustable_mount",
        width_mm=5.0,
        length_mm=28.0,
        cut_depth_mm=length + 4,
        create_geometry=True,
    )

    for feature_id, y, z, diameter in cosmetic_holes:
        design.clearance_hole(
            feature_id=feature_id,
            part="plate",
            fastener="none",
            diameter_mm=diameter,
            center=(0, y, z),
            axis=(1, 0, 0),
            role="visual_lightening",
            intent="cosmetic",
        )
        with Locations((0, y, z)):
            Cylinder(
                radius=diameter / 2.0,
                height=length + 4,
                rotation=(0, 90, 0),
                mode=Mode.SUBTRACT,
            )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
