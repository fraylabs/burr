from pathlib import Path

from build123d import Box, BuildPart, export_step
from burr_build123d import (
    BurrDesignData,
    DESIGN_DATA_FILE,
    heat_set_insert_pocket,
    m3_clearance_hole,
)


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "electronics-standoff-deck.step"

length = 20.0
width = 70.0
height = 34.0

design = BurrDesignData(
    artifact_id="gallery-electronics-standoff-deck",
    artifact_type="actuator_mount",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "deck",
    bbox_min=(-length / 2, -width / 2, -height / 2),
    bbox_max=(length / 2, width / 2, height / 2),
)


with BuildPart() as part:
    Box(length, width, height)

    for y in (-22.0, 22.0):
        heat_set_insert_pocket(
            design,
            feature_id=f"m3_insert_socket_{'left' if y < 0 else 'right'}",
            part="deck",
            center=(0, y, 7.0),
            axis=(1, 0, 0),
            role="pcb_standoff",
            insert="M3x5x4",
            pocket_diameter_mm=4.7,
            pocket_depth_mm=5.0,
            host_depth_mm=length,
            create_geometry=True,
        )

    heat_set_insert_pocket(
        design,
        feature_id="cosmetic_label_socket",
        part="deck",
        center=(0, 0, 7.0),
        axis=(1, 0, 0),
        role="cosmetic",
        intent="cosmetic",
        insert="label-dot",
        pocket_diameter_mm=3.0,
        pocket_depth_mm=2.0,
        host_depth_mm=length,
        create_geometry=True,
    )

    for y in (-22.0, 22.0):
        m3_clearance_hole(
            design,
            feature_id=f"m3_base_mount_{'left' if y < 0 else 'right'}",
            part="deck",
            center=(0, y, -4.0),
            axis=(1, 0, 0),
            role="mount",
            diameter_mm=3.4,
            cut_depth_mm=length + 4,
            create_geometry=True,
        )

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
