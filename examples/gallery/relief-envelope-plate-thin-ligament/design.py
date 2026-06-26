from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, spacing_envelope


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "relief-envelope-plate-thin-ligament.step"

length = 14.0
width = 80.0
height = 42.0
relief_start = (0, -8.0, 0.0)
relief_end = (0, 8.0, 0.0)
relief_radius = 3.0

design = BurrDesignData(
    artifact_id="gallery-relief-envelope-plate-thin-ligament",
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
    ("near_relief_hole", 13.2, 0.0, 4.0),
    ("relief_grid_a1", -30.0, -14.0, 2.6),
    ("relief_grid_a2", -18.0, -14.0, 2.6),
    ("relief_grid_a3", 18.0, -14.0, 2.6),
    ("relief_grid_a4", 30.0, -14.0, 2.6),
    ("relief_grid_b1", -30.0, 14.0, 2.6),
    ("relief_grid_b2", -18.0, 14.0, 2.6),
    ("relief_grid_b3", 30.0, 14.0, 2.6),
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


def cut_relief_capsule() -> None:
    start_y = relief_start[1]
    end_y = relief_end[1]
    center_y = (start_y + end_y) / 2.0
    straight_length = abs(end_y - start_y)
    with Locations((0, center_y, relief_start[2])):
        Box(length + 4, straight_length, relief_radius * 2.0, mode=Mode.SUBTRACT)
    for y in (start_y, end_y):
        with Locations((0, y, relief_start[2])):
            Cylinder(
                radius=relief_radius,
                height=length + 4,
                rotation=(0, 90, 0),
                mode=Mode.SUBTRACT,
            )


with BuildPart() as part:
    Box(length, width, height)
    cut_relief_capsule()
    design.feature(
        feature_id="rounded_relief_window",
        kind="cutout",
        part="plate",
        intent="cosmetic",
        role="relief_slot",
        spacing_envelope=spacing_envelope(
            segment_start=relief_start,
            segment_end=relief_end,
            radius_mm=relief_radius,
        ),
    )

    for hole in cosmetic_holes:
        cut_cosmetic_hole(*hole)

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
