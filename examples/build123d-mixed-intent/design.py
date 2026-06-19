from pathlib import Path

from build123d import Box, BuildPart, Cylinder, Locations, Mode, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "mixed.step"

housing_length = 60.0
housing_width = 40.0
housing_height = 24.0

design = BurrDesignData(
    artifact_id="build123d-mixed-intent",
    artifact_type="actuator_mount",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "housing",
    bbox_min=(-housing_length / 2.0, -housing_width / 2.0, 0),
    bbox_max=(housing_length / 2.0, housing_width / 2.0, housing_height),
)


def x_axis_cut(center: tuple[float, float, float], diameter_mm: float) -> None:
    with Locations(center):
        Cylinder(
            radius=diameter_mm / 2.0,
            height=housing_length + 10.0,
            rotation=(0, 90, 0),
            mode=Mode.SUBTRACT,
        )


with BuildPart() as housing:
    with Locations((0, 0, housing_height / 2.0)):
        Box(housing_length, housing_width, housing_height)

    m3_clearance_hole(
        design,
        feature_id="m3_mount",
        part="housing",
        center=(0, -5, 12),
        axis=(1, 0, 0),
        role="loaded_mount",
        intent="mechanical_interface",
        diameter_mm=3.4,
        cut_depth_mm=housing_length + 10.0,
    )

    design.clearance_hole(
        feature_id="lightening_hole",
        part="housing",
        fastener="none",
        diameter_mm=7.0,
        center=(0, 9, 12),
        axis=(1, 0, 0),
        role="mass_reduction",
        intent="weight_reduction",
    )
    x_axis_cut((0, 9, 12), 7.0)

    design.clearance_hole(
        feature_id="air_passage",
        part="housing",
        fastener="none",
        diameter_mm=3.0,
        center=(0, -14, 12),
        axis=(1, 0, 0),
        role="vent",
        intent="fluid_or_air_path",
    )
    x_axis_cut((0, -14, 12), 3.0)

    design.clearance_hole(
        feature_id="cosmetic_dot",
        part="housing",
        fastener="none",
        diameter_mm=2.4,
        center=(0, 0, 4),
        axis=(1, 0, 0),
        role="cosmetic",
        intent="cosmetic",
    )
    x_axis_cut((0, 0, 4), 2.4)

export_step(housing.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)

print(f"wrote {BASE_DIR / DESIGN_DATA_FILE}")
