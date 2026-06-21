from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, bearing_seat, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "shaft-bearing-bracket.step"

length = 22.0
width = 58.0
height = 38.0

design = BurrDesignData(
    artifact_id="gallery-shaft-bearing-bracket",
    artifact_type="actuator_mount",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "bracket",
    bbox_min=(-length / 2, -width / 2, -height / 2),
    bbox_max=(length / 2, width / 2, height / 2),
)


with BuildPart() as part:
    Box(length, width, height)

    bearing_seat(
        design,
        feature_id="bearing_608_primary",
        part="bracket",
        center=(0, 0, 0),
        axis=(1, 0, 0),
        role="bearing_support",
        bearing="608",
        seat_diameter_mm=22.0,
        seat_depth_mm=7.0,
        host_depth_mm=length,
        create_geometry=True,
    )

    bearing_seat(
        design,
        feature_id="cosmetic_relief_recess",
        part="bracket",
        center=(0, 0, 14.0),
        axis=(1, 0, 0),
        role="cosmetic",
        intent="cosmetic",
        bearing="decorative-relief",
        seat_diameter_mm=8.0,
        seat_depth_mm=3.0,
        host_depth_mm=length,
        create_geometry=True,
    )

    for y in (-18.0, 18.0):
        m3_clearance_hole(
            design,
            feature_id=f"m3_loaded_mount_{'left' if y < 0 else 'right'}",
            part="bracket",
            center=(0, y, 0),
            axis=(1, 0, 0),
            role="loaded_mount",
            diameter_mm=3.4,
            cut_depth_mm=length + 4,
            create_geometry=True,
        )

    design.clearance_hole(
        feature_id="wire_passage",
        part="bracket",
        fastener="none",
        diameter_mm=5.0,
        center=(0, 0, -14.0),
        axis=(1, 0, 0),
        role="routing",
        intent="fluid_or_air_path",
    )
    with Locations((0, 0, -14.0)):
        Cylinder(radius=2.5, height=length + 4, rotation=(0, 90, 0), mode=Mode.SUBTRACT)

export_step(part.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
