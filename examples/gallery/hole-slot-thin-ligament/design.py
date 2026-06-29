from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, straight_slot


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "hole-slot-thin-ligament.step"

length = 14.0
width = 86.0
height = 48.0

design = BurrDesignData(
    artifact_id="gallery-hole-slot-thin-ligament",
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
    ("near_slot_hole", 5.0, 0.0, 4.0),
    ("relief_a1", -36.0, -17.0, 3.0),
    ("relief_a2", -24.0, -17.0, 3.0),
    ("relief_a3", -12.0, -17.0, 3.0),
    ("relief_a4", 12.0, -17.0, 3.0),
    ("relief_a5", 24.0, -17.0, 3.0),
    ("relief_a6", 36.0, -17.0, 3.0),
    ("relief_b1", -30.0, 17.0, 3.0),
]


def cut_cosmetic_hole(feature_id: str, y: float, z: float, diameter: float) -> None:
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


with BuildPart() as part:
    Box(length, width, height)
    straight_slot(
        design,
        feature_id="long_relief_slot",
        part="plate",
        center=(0, 0, 0),
        axis=(1, 0, 0),
        span_axis=(0, 1, 0),
        role="relief_slot",
        intent="cosmetic",
        width_mm=5.0,
        length_mm=30.0,
        cut_depth_mm=length + 4,
        create_geometry=True,
    )
    for hole in cosmetic_holes:
        cut_cosmetic_hole(*hole)

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
