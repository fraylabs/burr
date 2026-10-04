"""Parametric source for the Burr fold-flat travel hanger example."""

from __future__ import annotations

from math import cos, radians, sin

from build123d import (
    Align,
    Box,
    BuildPart,
    BuildSketch,
    Color,
    Compound,
    Cylinder,
    Location,
    Locations,
    Mode,
    Plane,
    Polygon,
    extrude,
)


# Coordinate convention: hanger lies in XY; +Y points toward the hook; +Z is
# thickness. Each arm is authored from a hinge at its local origin along +X.
HUB_WIDTH = 86.0
HUB_HEIGHT = 22.0
HUB_THICKNESS = 4.0
HINGE_X = 26.0

HOOK_CENTER_Y = 38.0
HOOK_OUTER_RADIUS = 22.0
HOOK_INNER_RADIUS = 14.0
HOOK_OPENING_X = 6.5

PIN_RADIUS = 3.0
PIN_HEIGHT = 7.6
PIN_BOTTOM_Z = HUB_THICKNESS / 2.0
ARM_HOLE_RADIUS = 3.4
RADIAL_CLEARANCE = ARM_HOLE_RADIUS - PIN_RADIUS

ARM_LENGTH = 212.0
ARM_ROOT_HALF_WIDTH = 9.0
ARM_TIP_HALF_WIDTH = 4.5
ARM_THICKNESS = 6.0
ARM_BOTTOM_Z = HUB_THICKNESS / 2.0 + 0.6
AXIAL_CLEARANCE = ARM_BOTTOM_Z - HUB_THICKNESS / 2.0

# The check models use intentionally simple closed solids. They cover the hub
# body and the load-bearing span of each arm, but omit the hook, pins, bores,
# and stops whose curved STEP faces are not yet tessellated reliably by Burr's
# viewer dependency.
CHECK_ARM_START = 10.0
CHECK_ARM_WIDTH = 14.0

DEPLOYED_RIGHT_ANGLE = -15.0
DEPLOYED_LEFT_ANGLE = 195.0
FOLDED_RIGHT_ANGLE = -85.0
FOLDED_LEFT_ANGLE = -95.0

STOP_RADIUS = 1.5
STOP_LOCAL_X = 12.0
STOP_LOCAL_Y = ARM_ROOT_HALF_WIDTH + STOP_RADIUS


def _linear_channel(hex_color: str) -> Color:
    """Convert an sRGB hex colour to the linear channels build123d expects."""

    channels = []
    for offset in (1, 3, 5):
        value = int(hex_color[offset : offset + 2], 16) / 255.0
        channels.append(value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4)
    return Color(*channels)


def _stop_world(pivot_x: float, angle_degrees: float, side: float) -> tuple[float, float]:
    """Return a fixed stop location tangent to an arm in its deployed pose."""

    angle = radians(angle_degrees)
    local_y = side * STOP_LOCAL_Y
    x = pivot_x + STOP_LOCAL_X * cos(angle) - local_y * sin(angle)
    y = STOP_LOCAL_X * sin(angle) + local_y * cos(angle)
    return (x, y)


def make_hub() -> object:
    """Create the fixed hub, open hook, hinge pins, and deployed-position stops."""

    right_stop = _stop_world(HINGE_X, DEPLOYED_RIGHT_ANGLE, 1.0)
    left_stop = _stop_world(-HINGE_X, DEPLOYED_LEFT_ANGLE, -1.0)

    with BuildPart() as hub:
        Box(
            HUB_WIDTH,
            HUB_HEIGHT,
            HUB_THICKNESS,
            align=(Align.CENTER, Align.CENTER, Align.CENTER),
        )
        with Locations((0.0, 16.0, 0.0)):
            Box(
                18.0,
                30.0,
                HUB_THICKNESS,
                align=(Align.CENTER, Align.CENTER, Align.CENTER),
            )

        with Locations((0.0, HOOK_CENTER_Y, -HUB_THICKNESS / 2.0)):
            Cylinder(
                HOOK_OUTER_RADIUS,
                HUB_THICKNESS,
                align=(Align.CENTER, Align.CENTER, Align.MIN),
            )
            Cylinder(
                HOOK_INNER_RADIUS,
                HUB_THICKNESS + 2.0,
                align=(Align.CENTER, Align.CENTER, Align.MIN),
                mode=Mode.SUBTRACT,
            )

        with Locations((HOOK_OPENING_X + 12.0, HOOK_CENTER_Y - 1.0, 0.0)):
            Box(
                24.0,
                27.0,
                HUB_THICKNESS + 2.0,
                align=(Align.CENTER, Align.CENTER, Align.CENTER),
                mode=Mode.SUBTRACT,
            )

        for x in (-HINGE_X, HINGE_X):
            with Locations((x, 0.0, PIN_BOTTOM_Z)):
                Cylinder(
                    PIN_RADIUS,
                    PIN_HEIGHT,
                    align=(Align.CENTER, Align.CENTER, Align.MIN),
                )

        for x, y in (left_stop, right_stop):
            with Locations((x, y, PIN_BOTTOM_Z)):
                Cylinder(
                    STOP_RADIUS,
                    PIN_HEIGHT - 1.0,
                    align=(Align.CENTER, Align.CENTER, Align.MIN),
                )

    part = hub.part
    part.label = "hook_and_hinge_hub"
    part.color = _linear_channel("#F2A65A")
    return part


def make_arm(label: str) -> object:
    """Create one tapered hanger arm with a print-clearance hinge bore."""

    profile = [
        (0.0, -ARM_ROOT_HALF_WIDTH),
        (ARM_LENGTH - 30.0, -7.0),
        (ARM_LENGTH, -ARM_TIP_HALF_WIDTH),
        (ARM_LENGTH, ARM_TIP_HALF_WIDTH),
        (ARM_LENGTH - 30.0, 7.0),
        (0.0, ARM_ROOT_HALF_WIDTH),
    ]

    with BuildPart() as arm:
        Cylinder(
            ARM_ROOT_HALF_WIDTH + 1.0,
            ARM_THICKNESS,
            align=(Align.CENTER, Align.CENTER, Align.MIN),
        )
        with BuildSketch(Plane.XY):
            Polygon(profile, align=None)
        extrude(amount=ARM_THICKNESS)
        with Locations((0.0, 0.0, -1.0)):
            Cylinder(
                ARM_HOLE_RADIUS,
                ARM_THICKNESS + 2.0,
                align=(Align.CENTER, Align.CENTER, Align.MIN),
                mode=Mode.SUBTRACT,
            )

    part = arm.part
    part.label = label
    part.color = _linear_channel("#5BA7D1")
    return part


def _placed_arm(label: str, pivot_x: float, angle_degrees: float) -> object:
    arm = make_arm(label)
    return arm.moved(Location((pivot_x, 0.0, ARM_BOTTOM_Z), (0.0, 0.0, angle_degrees)))


def make_hanger(*, folded: bool) -> Compound:
    """Create one static pose from the shared hinge geometry."""

    left_angle = FOLDED_LEFT_ANGLE if folded else DEPLOYED_LEFT_ANGLE
    right_angle = FOLDED_RIGHT_ANGLE if folded else DEPLOYED_RIGHT_ANGLE
    pose = "folded" if folded else "deployed"

    assembly = Compound(
        label=f"fold_flat_travel_hanger:{pose}",
        children=[
            make_hub(),
            _placed_arm("left_folding_arm", -HINGE_X, left_angle),
            _placed_arm("right_folding_arm", HINGE_X, right_angle),
        ],
    )
    return assembly


def _make_check_hub() -> object:
    """Create the closed hub-body envelope used by Burr's current check."""

    part = Box(
        HUB_WIDTH,
        HUB_HEIGHT,
        HUB_THICKNESS,
        align=(Align.CENTER, Align.CENTER, Align.CENTER),
    )
    part.label = "hub_body_envelope"
    part.color = _linear_channel("#F2A65A")
    return part


def _make_check_arm(label: str, pivot_x: float, angle_degrees: float) -> object:
    """Create a closed envelope for the load-bearing span of one arm."""

    part = Box(
        ARM_LENGTH - CHECK_ARM_START,
        CHECK_ARM_WIDTH,
        ARM_THICKNESS,
        align=(Align.MIN, Align.CENTER, Align.MIN),
    )
    part.label = label
    part.color = _linear_channel("#5BA7D1")
    return part.moved(
        Location(
            (pivot_x, 0.0, ARM_BOTTOM_Z),
            (0.0, 0.0, angle_degrees),
        )
        * Location((CHECK_ARM_START, 0.0, 0.0))
    )


def make_hanger_check_envelope(*, folded: bool) -> Compound:
    """Create the documented closed envelopes used for interference checks."""

    left_angle = FOLDED_LEFT_ANGLE if folded else DEPLOYED_LEFT_ANGLE
    right_angle = FOLDED_RIGHT_ANGLE if folded else DEPLOYED_RIGHT_ANGLE
    pose = "folded" if folded else "deployed"

    return Compound(
        label=f"fold_flat_travel_hanger:{pose}:check_envelope",
        children=[
            _make_check_hub(),
            _make_check_arm("left_arm_body_envelope", -HINGE_X, left_angle),
            _make_check_arm("right_arm_body_envelope", HINGE_X, right_angle),
        ],
    )
