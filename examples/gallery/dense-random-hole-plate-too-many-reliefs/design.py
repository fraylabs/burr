from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "dense-random-hole-plate-too-many-reliefs.step"

length = 12.0
width = 126.0
height = 126.0
hole_diameter = 2.6

design = BurrDesignData(
    artifact_id="gallery-dense-random-hole-plate-too-many-reliefs",
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

hole_positions = []
for y in range(-48, 49, 12):
    for z in range(-48, 49, 12):
        hole_positions.append((float(y), float(z)))

with BuildPart() as part:
    Box(length, width, height)
    for index, (y, z) in enumerate(hole_positions, start=1):
        design.clearance_hole(
            feature_id=f"relief_{index:02d}",
            part="plate",
            fastener="none",
            diameter_mm=hole_diameter,
            center=(0, y, z),
            axis=(1, 0, 0),
            role="visual_lightening",
            intent="cosmetic",
        )
        with Locations((0, y, z)):
            Cylinder(
                radius=hole_diameter / 2.0,
                height=length + 4,
                rotation=(0, 90, 0),
                mode=Mode.SUBTRACT,
            )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
