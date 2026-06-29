from pathlib import Path

from build123d import Box, BuildPart, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "practical-snap-hook-thin.step"

depth = 12.0
width = 36.0
height = 4.0
hook_thickness = 0.7
hook_engagement = 1.0

design = BurrDesignData(
    artifact_id="gallery-practical-snap-hook-thin",
    artifact_type="printable_retention_clip",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/printable_retention.rulepack.json")
design.measurements_update(
    {
        "snap_hook_arm_thickness_mm": hook_thickness,
        "snap_hook_engagement_mm": hook_engagement,
    },
)
design.part(
    "clip",
    bbox_min=(-depth / 2.0, -width / 2.0, 0.0),
    bbox_max=(depth / 2.0, width / 2.0, height + 8.0),
)
for side, y in (("left", -12.0), ("right", 12.0)):
    design.feature(
        feature_id=f"{side}_snap_hook",
        part="clip",
        kind="snap_hook",
        role="retention",
        thickness_mm=hook_thickness,
        engagement_mm=hook_engagement,
    )

with BuildPart() as part:
    with Locations((0, 0, height / 2.0)):
        Box(depth, width, height)
    for y in (-12.0, 12.0):
        with Locations((0, y, height + 3.0)):
            Box(depth, hook_thickness, 6.0, mode=Mode.ADD)
        with Locations((0, y + (hook_thickness + hook_engagement), height + 5.5)):
            Box(depth, hook_engagement * 2.0, hook_thickness, mode=Mode.ADD)

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
