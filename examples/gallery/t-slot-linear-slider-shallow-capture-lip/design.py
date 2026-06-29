from pathlib import Path

from build123d import Box, BuildPart, Compound, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "t-slot-linear-slider-shallow-capture-lip.step"

rail_length = 40.0
rail_head_width = 10.0
rail_head_height = 3.0
rail_neck_width = 5.0
rail_neck_height = 4.0
clearance = 0.25
carriage_length = 16.0
lip_each_side = 1.0
carriage_outer_width = rail_head_width + 2.0 * clearance + 2.0 * lip_each_side
carriage_outer_height = 8.0

design = BurrDesignData(
    artifact_id="gallery-t-slot-linear-slider-shallow-capture-lip",
    artifact_type="captured_slider",
    artifact_version="0.1.0",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.rulepack("../../../rules/captured_slider.rulepack.json")
design.measurements_update(
    {
        "head_side_clearance_mm": clearance,
        "neck_side_clearance_mm": clearance,
        "carriage_lip_each_side_mm": lip_each_side,
    },
)
design.part(
    "rail",
    bbox_min=(-rail_head_width / 2.0, -rail_length / 2.0, 0.0),
    bbox_max=(rail_head_width / 2.0, rail_length / 2.0, rail_neck_height + rail_head_height),
)
design.part(
    "carriage",
    bbox_min=(-carriage_outer_width / 2.0, -carriage_length / 2.0, 0.0),
    bbox_max=(carriage_outer_width / 2.0, carriage_length / 2.0, carriage_outer_height),
)
for side in ("left", "right"):
    design.feature(
        feature_id=f"{side}_capture_lip",
        part="carriage",
        kind="capture_lip",
        role="lift_off_blocker",
        engagement_mm=lip_each_side,
    )

with BuildPart() as rail:
    with Locations((0, 0, rail_neck_height / 2.0)):
        Box(rail_neck_width, rail_length, rail_neck_height)
    with Locations((0, 0, rail_neck_height + rail_head_height / 2.0)):
        Box(rail_head_width, rail_length, rail_head_height)

with BuildPart() as carriage:
    with Locations((0, 0, carriage_outer_height / 2.0)):
        Box(carriage_outer_width, carriage_length, carriage_outer_height)
    with Locations((0, 0, rail_neck_height / 2.0)):
        Box(
            rail_neck_width + 2.0 * clearance,
            carriage_length + 2.0,
            rail_neck_height + clearance,
            mode=Mode.SUBTRACT,
        )
    with Locations((0, 0, rail_neck_height + rail_head_height / 2.0)):
        Box(
            rail_head_width + 2.0 * clearance,
            carriage_length + 2.0,
            rail_head_height + 2.0 * clearance,
            mode=Mode.SUBTRACT,
        )
    for y in (-5.0, 5.0):
        with Locations((0, y, carriage_outer_height / 2.0)):
            Cylinder(radius=1.4, height=carriage_outer_height + 2.0, mode=Mode.SUBTRACT)

assembly = Compound(children=[rail.part, carriage.part])
export_step(assembly, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)
