from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, heat_set_insert_pocket


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "practical-insert-fit-tight.step"

host_depth = 14.0
width = 26.0
height = 20.0
insert_nominal_diameter = 4.6
pocket_diameter = 4.66
pocket_depth = 6.4

design = BurrDesignData(
    artifact_id="gallery-practical-insert-fit-tight",
    artifact_type="hardware_fit_plate",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/hardware_fit.rulepack.json")
design.measurements_update(
    {
        "insert_pocket_radial_clearance_mm": (pocket_diameter - insert_nominal_diameter) / 2.0,
        "insert_pocket_depth_margin_mm": pocket_depth - 5.7,
    },
)
design.part(
    "plate",
    bbox_min=(-host_depth / 2.0, -width / 2.0, -height / 2.0),
    bbox_max=(host_depth / 2.0, width / 2.0, height / 2.0),
)

with BuildPart() as part:
    Box(host_depth, width, height)
    heat_set_insert_pocket(
        design,
        feature_id="m3_insert_pocket",
        part="plate",
        center=(0, 0, 0),
        axis=(1, 0, 0),
        role="threaded_mount",
        insert="M3x5.7",
        pocket_diameter_mm=pocket_diameter,
        pocket_depth_mm=pocket_depth,
        host_depth_mm=host_depth,
        support_diameter_mm=9.0,
        create_geometry=True,
    )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
