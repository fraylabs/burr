from pathlib import Path

from build123d import Box, BuildPart, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, standoff_boss


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "practical-boss-support-unsupported.step"

plate_depth = 38.0
plate_width = 28.0
plate_height = 4.0
boss_diameter = 5.0
boss_height = 16.0

design = BurrDesignData(
    artifact_id="gallery-practical-boss-support-unsupported",
    artifact_type="boss_support_plate",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/boss_support.rulepack.json")
design.measurement("boss_height_to_diameter_ratio", boss_height / boss_diameter)
design.part(
    "plate",
    bbox_min=(-plate_depth / 2.0, -plate_width / 2.0, 0.0),
    bbox_max=(plate_depth / 2.0, plate_width / 2.0, plate_height + boss_height),
)
design.feature(
    feature_id="single_stub_gusset",
    part="plate",
    kind="rib",
    role="boss_support",
    thickness_mm=1.0,
)

with BuildPart() as part:
    with Locations((0, 0, plate_height / 2.0)):
        Box(plate_depth, plate_width, plate_height)
    standoff_boss(
        design,
        feature_id="m3_boss",
        part="plate",
        center=(0, 0, plate_height + boss_height / 2.0),
        axis=(0, 0, 1),
        role="bossed_mount",
        boss_diameter_mm=boss_diameter,
        boss_height_mm=boss_height,
        create_geometry=True,
    )
    with Locations((0, -4.0, plate_height + boss_height / 4.0)):
        Box(plate_depth * 0.3, 1.0, boss_height * 0.45, mode=Mode.ADD)

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
