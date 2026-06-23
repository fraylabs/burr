from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "fastener-support.step"

plate_length = 42.0
plate_width = 28.0
plate_thickness = 4.0
boss_diameter = 8.0
boss_height = 5.0
m3_diameter = 3.4

design = BurrDesignData(
    artifact_id="build123d-fastener-support-good",
    artifact_type="actuator_mount",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "plate",
    bbox_min=(-plate_length / 2, -plate_width / 2, 0),
    bbox_max=(plate_length / 2, plate_width / 2, plate_thickness + boss_height),
)

with BuildPart() as part:
    with Locations((0, 0, plate_thickness / 2)):
        Box(plate_length, plate_width, plate_thickness)

    with Locations((0, 0, plate_thickness + boss_height / 2)):
        Cylinder(radius=boss_diameter / 2, height=boss_height, mode=Mode.ADD)

    m3_clearance_hole(
        design,
        feature_id="m3_bossed_mount",
        part="plate",
        center=(0, 0, (plate_thickness + boss_height) / 2),
        axis=(0, 0, 1),
        role="bossed_mount",
        diameter_mm=m3_diameter,
        support_diameter_mm=boss_diameter,
        cut_depth_mm=plate_thickness + boss_height + 2,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
