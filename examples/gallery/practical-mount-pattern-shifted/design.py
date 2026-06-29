from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "practical-mount-pattern-shifted.step"

depth = 10.0
width = 42.0
height = 30.0
hole_centers = [(-12.0, -8.0), (12.0, -8.0), (-12.0, 8.0), (12.8, 8.0)]

design = BurrDesignData(
    artifact_id="gallery-practical-mount-pattern-shifted",
    artifact_type="mount_pattern_plate",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/mount_pattern.rulepack.json")
design.measurement("mount_pattern_max_pitch_error_mm", 0.8)
design.part(
    "plate",
    bbox_min=(-depth / 2.0, -width / 2.0, -height / 2.0),
    bbox_max=(depth / 2.0, width / 2.0, height / 2.0),
)

with BuildPart() as part:
    Box(depth, width, height)
    for index, (y, z) in enumerate(hole_centers, start=1):
        m3_clearance_hole(
            design,
            feature_id=f"pattern_m3_{index}",
            part="plate",
            center=(0, y, z),
            axis=(1, 0, 0),
            role="pattern_mount",
            create_geometry=True,
            cut_depth_mm=depth + 4,
        )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
