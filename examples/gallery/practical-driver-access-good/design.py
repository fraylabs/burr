from pathlib import Path

from build123d import Box, BuildPart, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "practical-driver-access-good.step"

depth = 12.0
width = 34.0
height = 22.0
driver_access_diameter = 7.0
driver_side_clearance = 2.0

design = BurrDesignData(
    artifact_id="gallery-practical-driver-access-good",
    artifact_type="tool_access_mount",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/tool_access.rulepack.json")
design.measurements_update(
    {
        "driver_access_diameter_mm": driver_access_diameter,
        "driver_side_clearance_mm": driver_side_clearance,
    },
)
design.part(
    "mount",
    bbox_min=(-depth / 2.0, -width / 2.0, -height / 2.0),
    bbox_max=(depth / 2.0, width / 2.0, height / 2.0 + 5.0),
)
design.feature(
    feature_id="driver_access_envelope",
    part="mount",
    kind="tool_access_envelope",
    role="assembly_access",
    diameter_mm=driver_access_diameter,
)

with BuildPart() as part:
    Box(depth, width, height)
    m3_clearance_hole(
        design,
        feature_id="m3_service_screw",
        part="mount",
        center=(0, 0, 0),
        axis=(1, 0, 0),
        role="housing_mount",
        create_geometry=True,
        cut_depth_mm=depth + 4,
    )
    with Locations((0, width / 2.0 - 2.0, 0)):
        Box(depth, 4.0, height + 10.0, mode=Mode.ADD)

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
