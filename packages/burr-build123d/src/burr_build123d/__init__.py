"""Small build123d helpers for emitting Burr design data.

This package does not replace build123d. It records mechanical intent while a
normal build123d design file creates geometry.
"""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


DESIGN_DATA_FILE = "burr-design-data.json"
DESIGN_DATA_SCHEMA = "burr.design-data.v1"
__version__ = "0.8.0"


def _round_vector(values: tuple[float, float, float] | list[float]) -> list[float]:
    return [round(float(value), 6) for value in values]


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _stamp_ref(base_dir: Path, ref: dict[str, Any]) -> None:
    path = ref.get("path")
    if not path:
        return
    file_path = base_dir / path
    if not file_path.exists():
        return
    ref["sha256"] = _sha256_file(file_path)
    ref["size_bytes"] = file_path.stat().st_size


@dataclass
class BurrDesignData:
    """Records the design facts Burr checks.

    The editable design file owns this object. Helpers add parts/features while
    build123d creates the actual geometry.
    """

    artifact_id: str
    artifact_type: str
    artifact_version: str = "0.1.0"
    units: str = "mm"
    process: dict[str, Any] | None = None
    sources: list[dict[str, Any]] = field(default_factory=list)
    artifacts: list[dict[str, Any]] = field(default_factory=list)
    parts: list[dict[str, Any]] = field(default_factory=list)
    features: list[dict[str, Any]] = field(default_factory=list)
    rulepack_ref: dict[str, Any] | None = None
    measurements: dict[str, float] = field(default_factory=dict)

    def source(self, path: str, kind: str = "design_file") -> None:
        self.sources.append({"kind": kind, "path": path})

    def artifact(self, path: str, kind: str = "step") -> None:
        self.artifacts.append({"kind": kind, "path": path})

    def rulepack(self, path: str) -> None:
        if not path:
            raise ValueError("rulepack path must be a non-empty string.")
        self.rulepack_ref = {"path": path}

    def measurement(self, name: str, value: float) -> None:
        if not name:
            raise ValueError("measurement name must be a non-empty string.")
        value = float(value)
        if not math.isfinite(value):
            raise ValueError("measurement value must be finite.")
        self.measurements[name] = value

    def measurements_update(self, values: dict[str, float]) -> None:
        for name, value in values.items():
            self.measurement(name, value)

    def part(
        self,
        part_id: str,
        *,
        bbox_min: tuple[float, float, float] | list[float],
        bbox_max: tuple[float, float, float] | list[float],
    ) -> None:
        self.parts.append(
            {
                "id": part_id,
                "bbox_mm": {
                    "min": _round_vector(bbox_min),
                    "max": _round_vector(bbox_max),
                },
            },
        )

    def feature(
        self,
        *,
        feature_id: str,
        kind: str,
        part: str | None = None,
        intent: str = "mechanical_interface",
        role: str | list[str] | None = None,
        **fields: Any,
    ) -> None:
        if not feature_id:
            raise ValueError("feature_id must be a non-empty string.")
        if not kind:
            raise ValueError("feature kind must be a non-empty string.")
        if not intent:
            raise ValueError("feature intent must be a non-empty string.")
        reserved = {"id", "kind", "intent", "part", "role"}
        conflicts = reserved.intersection(fields)
        if conflicts:
            names = ", ".join(sorted(conflicts))
            raise ValueError(f"feature fields cannot override reserved keys: {names}.")
        feature: dict[str, Any] = {
            "id": feature_id,
            "kind": kind,
            "intent": intent,
            **fields,
        }
        if part is not None:
            feature["part"] = part
        if role is not None:
            feature["role"] = role
        self.features.append(feature)

    def clearance_hole(
        self,
        *,
        feature_id: str,
        part: str,
        fastener: str,
        diameter_mm: float,
        center: tuple[float, float, float] | list[float],
        axis: tuple[float, float, float] | list[float],
        role: str,
        intent: str = "mechanical_interface",
        support_diameter_mm: float | None = None,
    ) -> None:
        if not intent:
            raise ValueError("clearance_hole intent must be a non-empty string.")
        if support_diameter_mm is not None and support_diameter_mm <= diameter_mm:
            raise ValueError("clearance_hole support_diameter_mm must be greater than diameter_mm.")
        feature = {
            "id": feature_id,
            "part": part,
            "kind": "clearance_hole",
            "intent": intent,
            "fastener": fastener,
            "diameter_mm": float(diameter_mm),
            "center_mm": _round_vector(center),
            "axis": _round_vector(axis),
            "role": role,
        }
        if support_diameter_mm is not None:
            feature["support_diameter_mm"] = float(support_diameter_mm)
        self.features.append(feature)

    def straight_slot(
        self,
        *,
        feature_id: str,
        part: str,
        width_mm: float,
        length_mm: float,
        center: tuple[float, float, float] | list[float],
        axis: tuple[float, float, float] | list[float],
        span_axis: tuple[float, float, float] | list[float],
        role: str,
        intent: str = "mechanical_interface",
    ) -> None:
        if not intent:
            raise ValueError("straight_slot intent must be a non-empty string.")
        if width_mm <= 0:
            raise ValueError("straight_slot width_mm must be positive.")
        if length_mm <= width_mm:
            raise ValueError("straight_slot length_mm must be greater than width_mm.")
        self.features.append(
            {
                "id": feature_id,
                "part": part,
                "kind": "straight_slot",
                "intent": intent,
                "width_mm": float(width_mm),
                "length_mm": float(length_mm),
                "center_mm": _round_vector(center),
                "axis": _round_vector(axis),
                "span_axis": _round_vector(span_axis),
                "role": role,
            },
        )

    def counterbore(
        self,
        *,
        feature_id: str,
        part: str,
        bore_diameter_mm: float,
        counterbore_diameter_mm: float,
        counterbore_depth_mm: float,
        center: tuple[float, float, float] | list[float],
        axis: tuple[float, float, float] | list[float],
        counterbore_center: tuple[float, float, float] | list[float],
        shoulder_center: tuple[float, float, float] | list[float],
        role: str,
        intent: str = "mechanical_interface",
    ) -> None:
        if not intent:
            raise ValueError("counterbore intent must be a non-empty string.")
        if bore_diameter_mm <= 0:
            raise ValueError("counterbore bore_diameter_mm must be positive.")
        if counterbore_diameter_mm <= bore_diameter_mm:
            raise ValueError("counterbore counterbore_diameter_mm must be greater than bore_diameter_mm.")
        if counterbore_depth_mm <= 0:
            raise ValueError("counterbore counterbore_depth_mm must be positive.")
        self.features.append(
            {
                "id": feature_id,
                "part": part,
                "kind": "counterbore",
                "intent": intent,
                "bore_diameter_mm": float(bore_diameter_mm),
                "counterbore_diameter_mm": float(counterbore_diameter_mm),
                "counterbore_depth_mm": float(counterbore_depth_mm),
                "center_mm": _round_vector(center),
                "axis": _round_vector(axis),
                "counterbore_center_mm": _round_vector(counterbore_center),
                "shoulder_center_mm": _round_vector(shoulder_center),
                "role": role,
            },
        )

    def heat_set_insert_pocket(
        self,
        *,
        feature_id: str,
        part: str,
        insert: str,
        pocket_diameter_mm: float,
        pocket_depth_mm: float,
        center: tuple[float, float, float] | list[float],
        axis: tuple[float, float, float] | list[float],
        pocket_center: tuple[float, float, float] | list[float],
        bottom_center: tuple[float, float, float] | list[float],
        role: str,
        intent: str = "mechanical_interface",
        support_diameter_mm: float | None = None,
    ) -> None:
        if not intent:
            raise ValueError("heat_set_insert_pocket intent must be a non-empty string.")
        if pocket_diameter_mm <= 0:
            raise ValueError("heat_set_insert_pocket pocket_diameter_mm must be positive.")
        if pocket_depth_mm <= 0:
            raise ValueError("heat_set_insert_pocket pocket_depth_mm must be positive.")
        if support_diameter_mm is not None and support_diameter_mm <= pocket_diameter_mm:
            raise ValueError(
                "heat_set_insert_pocket support_diameter_mm must be greater than pocket_diameter_mm.",
            )
        feature = {
            "id": feature_id,
            "part": part,
            "kind": "heat_set_insert_pocket",
            "intent": intent,
            "insert": insert,
            "pocket_diameter_mm": float(pocket_diameter_mm),
            "pocket_depth_mm": float(pocket_depth_mm),
            "center_mm": _round_vector(center),
            "axis": _round_vector(axis),
            "pocket_center_mm": _round_vector(pocket_center),
            "bottom_center_mm": _round_vector(bottom_center),
            "role": role,
        }
        if support_diameter_mm is not None:
            feature["support_diameter_mm"] = float(support_diameter_mm)
        self.features.append(feature)

    def standoff_boss(
        self,
        *,
        feature_id: str,
        part: str,
        fastener: str,
        boss_diameter_mm: float,
        boss_height_mm: float,
        center: tuple[float, float, float] | list[float],
        axis: tuple[float, float, float] | list[float],
        boss_center: tuple[float, float, float] | list[float],
        top_center: tuple[float, float, float] | list[float],
        role: str,
        intent: str = "mechanical_interface",
        supports_feature_id: str | None = None,
    ) -> None:
        if not intent:
            raise ValueError("standoff_boss intent must be a non-empty string.")
        if boss_diameter_mm <= 0:
            raise ValueError("standoff_boss boss_diameter_mm must be positive.")
        if boss_height_mm <= 0:
            raise ValueError("standoff_boss boss_height_mm must be positive.")
        feature = {
            "id": feature_id,
            "part": part,
            "kind": "standoff_boss",
            "intent": intent,
            "fastener": fastener,
            "boss_diameter_mm": float(boss_diameter_mm),
            "boss_height_mm": float(boss_height_mm),
            "center_mm": _round_vector(center),
            "axis": _round_vector(axis),
            "boss_center_mm": _round_vector(boss_center),
            "top_center_mm": _round_vector(top_center),
            "role": role,
        }
        if supports_feature_id is not None:
            feature["supports_feature_id"] = supports_feature_id
        self.features.append(feature)

    def bearing_seat(
        self,
        *,
        feature_id: str,
        part: str,
        bearing: str,
        seat_diameter_mm: float,
        seat_depth_mm: float,
        center: tuple[float, float, float] | list[float],
        axis: tuple[float, float, float] | list[float],
        seat_center: tuple[float, float, float] | list[float],
        shoulder_center: tuple[float, float, float] | list[float],
        role: str,
        intent: str = "mechanical_interface",
    ) -> None:
        if not intent:
            raise ValueError("bearing_seat intent must be a non-empty string.")
        if seat_diameter_mm <= 0:
            raise ValueError("bearing_seat seat_diameter_mm must be positive.")
        if seat_depth_mm <= 0:
            raise ValueError("bearing_seat seat_depth_mm must be positive.")
        self.features.append(
            {
                "id": feature_id,
                "part": part,
                "kind": "bearing_seat",
                "intent": intent,
                "bearing": bearing,
                "seat_diameter_mm": float(seat_diameter_mm),
                "seat_depth_mm": float(seat_depth_mm),
                "center_mm": _round_vector(center),
                "axis": _round_vector(axis),
                "seat_center_mm": _round_vector(seat_center),
                "shoulder_center_mm": _round_vector(shoulder_center),
                "role": role,
            },
        )

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "schema_version": DESIGN_DATA_SCHEMA,
            "artifact_id": self.artifact_id,
            "artifact_version": self.artifact_version,
            "artifact_type": self.artifact_type,
            "units": self.units,
            "sources": self.sources,
            "artifacts": self.artifacts,
            "parts": self.parts,
            "features": self.features,
        }
        if self.sources:
            data["source"] = self.sources[0]
        if self.process:
            data["process"] = self.process
        if self.rulepack_ref:
            data["rulepack"] = self.rulepack_ref
        if self.measurements:
            data["measurements"] = self.measurements
        return data

    def write(self, path: str | Path) -> Path:
        output_path = Path(path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        data = self.to_dict()
        for ref in [data.get("source"), *data.get("sources", []), *data.get("artifacts", [])]:
            if ref:
                _stamp_ref(output_path.parent, ref)
        output_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        return output_path


def _axis_rotation(axis: tuple[float, float, float] | list[float]) -> tuple[float, float, float]:
    rounded = tuple(round(float(value), 6) for value in axis)
    if rounded == (1.0, 0.0, 0.0) or rounded == (-1.0, 0.0, 0.0):
        return (0, 90, 0)
    if rounded == (0.0, 1.0, 0.0) or rounded == (0.0, -1.0, 0.0):
        return (90, 0, 0)
    if rounded == (0.0, 0.0, 1.0) or rounded == (0.0, 0.0, -1.0):
        return (0, 0, 0)
    raise ValueError(f"Unsupported hole axis for build123d helper: {axis!r}")


def m3_clearance_hole(
    design: BurrDesignData,
    *,
    feature_id: str,
    part: str,
    center: tuple[float, float, float] | list[float],
    axis: tuple[float, float, float] | list[float],
    role: str,
    intent: str = "mechanical_interface",
    diameter_mm: float = 3.4,
    cut_depth_mm: float = 8.0,
    support_diameter_mm: float | None = None,
    create_geometry: bool = True,
) -> None:
    """Create a build123d cut and record an M3 clearance-hole feature."""

    design.clearance_hole(
        feature_id=feature_id,
        part=part,
        fastener="M3",
        diameter_mm=diameter_mm,
        center=center,
        axis=axis,
        role=role,
        intent=intent,
        support_diameter_mm=support_diameter_mm,
    )

    if not create_geometry:
        return

    try:
        from build123d import Cylinder, Locations, Mode
    except ImportError as error:
        raise RuntimeError(
            "m3_clearance_hole(create_geometry=True) requires build123d. "
            "Install build123d or pass create_geometry=False.",
        ) from error

    with Locations(tuple(center)):
        Cylinder(
            radius=diameter_mm / 2.0,
            height=cut_depth_mm,
            rotation=_axis_rotation(axis),
            mode=Mode.SUBTRACT,
        )


def straight_slot(
    design: BurrDesignData,
    *,
    feature_id: str,
    part: str,
    center: tuple[float, float, float] | list[float],
    axis: tuple[float, float, float] | list[float],
    span_axis: tuple[float, float, float] | list[float],
    role: str,
    width_mm: float,
    length_mm: float,
    cut_depth_mm: float,
    intent: str = "mechanical_interface",
    create_geometry: bool = True,
) -> None:
    """Create a narrow build123d straight slot cut and record slot intent.

    V1 supports slots cut along X and spanning along Y. This keeps the helper
    honest while Burr's verifier is still intentionally narrow.
    """

    design.straight_slot(
        feature_id=feature_id,
        part=part,
        width_mm=width_mm,
        length_mm=length_mm,
        center=center,
        axis=axis,
        span_axis=span_axis,
        role=role,
        intent=intent,
    )

    if not create_geometry:
        return

    if _round_vector(axis) != [1.0, 0.0, 0.0]:
        raise ValueError("straight_slot(create_geometry=True) currently requires axis=(1, 0, 0).")
    if _round_vector(span_axis) != [0.0, 1.0, 0.0]:
        raise ValueError(
            "straight_slot(create_geometry=True) currently requires span_axis=(0, 1, 0).",
        )

    try:
        from build123d import Box, Cylinder, Locations, Mode
    except ImportError as error:
        raise RuntimeError(
            "straight_slot(create_geometry=True) requires build123d. "
            "Install build123d or pass create_geometry=False.",
        ) from error

    straight_length = length_mm - width_mm
    center_x, center_y, center_z = tuple(center)
    endpoint_offset = straight_length / 2.0
    with Locations((center_x, center_y, center_z)):
        Box(cut_depth_mm, straight_length, width_mm, mode=Mode.SUBTRACT)
    for endpoint_y in (center_y - endpoint_offset, center_y + endpoint_offset):
        with Locations((center_x, endpoint_y, center_z)):
            Cylinder(
                radius=width_mm / 2.0,
                height=cut_depth_mm,
                rotation=(0, 90, 0),
                mode=Mode.SUBTRACT,
            )


def counterbore(
    design: BurrDesignData,
    *,
    feature_id: str,
    part: str,
    center: tuple[float, float, float] | list[float],
    axis: tuple[float, float, float] | list[float],
    role: str,
    bore_diameter_mm: float,
    counterbore_diameter_mm: float,
    counterbore_depth_mm: float,
    through_depth_mm: float,
    side: str = "negative",
    intent: str = "mechanical_interface",
    create_geometry: bool = True,
) -> None:
    """Create a narrow build123d counterbore cut and record counterbore intent.

    V1 supports counterbores cut along X from the negative or positive side.
    `through_depth_mm` should be the actual through-bore span of the part.
    """

    if _round_vector(axis) != [1.0, 0.0, 0.0]:
        raise ValueError("counterbore currently requires axis=(1, 0, 0).")
    if side not in {"negative", "positive"}:
        raise ValueError("counterbore side must be 'negative' or 'positive'.")
    if through_depth_mm <= counterbore_depth_mm:
        raise ValueError("counterbore through_depth_mm must be greater than counterbore_depth_mm.")

    center_x, center_y, center_z = tuple(center)
    side_sign = -1.0 if side == "negative" else 1.0
    counterbore_center = (
        center_x + side_sign * ((through_depth_mm - counterbore_depth_mm) / 2.0),
        center_y,
        center_z,
    )
    shoulder_center = (
        counterbore_center[0] - side_sign * (counterbore_depth_mm / 2.0),
        center_y,
        center_z,
    )

    design.counterbore(
        feature_id=feature_id,
        part=part,
        bore_diameter_mm=bore_diameter_mm,
        counterbore_diameter_mm=counterbore_diameter_mm,
        counterbore_depth_mm=counterbore_depth_mm,
        center=center,
        axis=axis,
        counterbore_center=counterbore_center,
        shoulder_center=shoulder_center,
        role=role,
        intent=intent,
    )

    if not create_geometry:
        return

    try:
        from build123d import Cylinder, Locations, Mode
    except ImportError as error:
        raise RuntimeError(
            "counterbore(create_geometry=True) requires build123d. "
            "Install build123d or pass create_geometry=False.",
        ) from error

    with Locations(tuple(center)):
        Cylinder(
            radius=bore_diameter_mm / 2.0,
            height=through_depth_mm,
            rotation=(0, 90, 0),
            mode=Mode.SUBTRACT,
        )
    with Locations(counterbore_center):
        Cylinder(
            radius=counterbore_diameter_mm / 2.0,
            height=counterbore_depth_mm,
            rotation=(0, 90, 0),
            mode=Mode.SUBTRACT,
        )


def heat_set_insert_pocket(
    design: BurrDesignData,
    *,
    feature_id: str,
    part: str,
    center: tuple[float, float, float] | list[float],
    axis: tuple[float, float, float] | list[float],
    role: str,
    insert: str,
    pocket_diameter_mm: float,
    pocket_depth_mm: float,
    host_depth_mm: float,
    side: str = "negative",
    intent: str = "mechanical_interface",
    support_diameter_mm: float | None = None,
    create_geometry: bool = True,
) -> None:
    """Create a narrow build123d blind insert pocket and record pocket intent.

    V1 supports blind pockets cut along X from the negative or positive side.
    Burr verifies the cylindrical pocket wall and the pocket bottom plane.
    """

    if _round_vector(axis) != [1.0, 0.0, 0.0]:
        raise ValueError("heat_set_insert_pocket currently requires axis=(1, 0, 0).")
    if side not in {"negative", "positive"}:
        raise ValueError("heat_set_insert_pocket side must be 'negative' or 'positive'.")
    if host_depth_mm <= pocket_depth_mm:
        raise ValueError("heat_set_insert_pocket host_depth_mm must be greater than pocket_depth_mm.")

    center_x, center_y, center_z = tuple(center)
    side_sign = -1.0 if side == "negative" else 1.0
    pocket_center = (
        center_x + side_sign * ((host_depth_mm - pocket_depth_mm) / 2.0),
        center_y,
        center_z,
    )
    bottom_center = (
        pocket_center[0] - side_sign * (pocket_depth_mm / 2.0),
        center_y,
        center_z,
    )

    design.heat_set_insert_pocket(
        feature_id=feature_id,
        part=part,
        insert=insert,
        pocket_diameter_mm=pocket_diameter_mm,
        pocket_depth_mm=pocket_depth_mm,
        center=center,
        axis=axis,
        pocket_center=pocket_center,
        bottom_center=bottom_center,
        role=role,
        intent=intent,
        support_diameter_mm=support_diameter_mm,
    )

    if not create_geometry:
        return

    try:
        from build123d import Cylinder, Locations, Mode
    except ImportError as error:
        raise RuntimeError(
            "heat_set_insert_pocket(create_geometry=True) requires build123d. "
            "Install build123d or pass create_geometry=False.",
        ) from error

    with Locations(pocket_center):
        Cylinder(
            radius=pocket_diameter_mm / 2.0,
            height=pocket_depth_mm,
            rotation=(0, 90, 0),
        mode=Mode.SUBTRACT,
    )


def standoff_boss(
    design: BurrDesignData,
    *,
    feature_id: str,
    part: str,
    center: tuple[float, float, float] | list[float],
    axis: tuple[float, float, float] | list[float],
    role: str,
    boss_diameter_mm: float,
    boss_height_mm: float,
    fastener: str = "M3",
    intent: str = "mechanical_interface",
    supports_feature_id: str | None = None,
    create_geometry: bool = True,
) -> None:
    """Create a raised build123d boss and record boss intent."""

    axis_vector = _round_vector(axis)
    if axis_vector not in ([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]):
        raise ValueError("standoff_boss currently requires an axis-aligned axis.")
    if boss_diameter_mm <= 0:
        raise ValueError("standoff_boss boss_diameter_mm must be positive.")
    if boss_height_mm <= 0:
        raise ValueError("standoff_boss boss_height_mm must be positive.")

    center_tuple = tuple(float(value) for value in center)
    axis_tuple = tuple(float(value) for value in axis_vector)
    top_center = tuple(
        center_tuple[index] + axis_tuple[index] * (boss_height_mm / 2.0)
        for index in range(3)
    )

    design.standoff_boss(
        feature_id=feature_id,
        part=part,
        fastener=fastener,
        boss_diameter_mm=boss_diameter_mm,
        boss_height_mm=boss_height_mm,
        center=center,
        axis=axis,
        boss_center=center,
        top_center=top_center,
        role=role,
        intent=intent,
        supports_feature_id=supports_feature_id,
    )

    if not create_geometry:
        return

    try:
        from build123d import Cylinder, Locations, Mode
    except ImportError as error:
        raise RuntimeError(
            "standoff_boss(create_geometry=True) requires build123d. "
            "Install build123d or pass create_geometry=False.",
        ) from error

    with Locations(center_tuple):
        Cylinder(
            radius=boss_diameter_mm / 2.0,
            height=boss_height_mm,
            rotation=_axis_rotation(axis),
            mode=Mode.ADD,
        )


def bearing_seat(
    design: BurrDesignData,
    *,
    feature_id: str,
    part: str,
    center: tuple[float, float, float] | list[float],
    axis: tuple[float, float, float] | list[float],
    role: str,
    bearing: str,
    seat_diameter_mm: float,
    seat_depth_mm: float,
    host_depth_mm: float,
    side: str = "negative",
    intent: str = "mechanical_interface",
    create_geometry: bool = True,
) -> None:
    """Create a narrow build123d blind bearing seat and record seat intent.

    V1 supports bearing seats cut along X from the negative or positive side.
    Burr verifies the cylindrical seat wall and the seat shoulder plane.
    """

    if _round_vector(axis) != [1.0, 0.0, 0.0]:
        raise ValueError("bearing_seat currently requires axis=(1, 0, 0).")
    if side not in {"negative", "positive"}:
        raise ValueError("bearing_seat side must be 'negative' or 'positive'.")
    if host_depth_mm <= seat_depth_mm:
        raise ValueError("bearing_seat host_depth_mm must be greater than seat_depth_mm.")

    center_x, center_y, center_z = tuple(center)
    side_sign = -1.0 if side == "negative" else 1.0
    seat_center = (
        center_x + side_sign * ((host_depth_mm - seat_depth_mm) / 2.0),
        center_y,
        center_z,
    )
    shoulder_center = (
        seat_center[0] - side_sign * (seat_depth_mm / 2.0),
        center_y,
        center_z,
    )

    design.bearing_seat(
        feature_id=feature_id,
        part=part,
        bearing=bearing,
        seat_diameter_mm=seat_diameter_mm,
        seat_depth_mm=seat_depth_mm,
        center=center,
        axis=axis,
        seat_center=seat_center,
        shoulder_center=shoulder_center,
        role=role,
        intent=intent,
    )

    if not create_geometry:
        return

    try:
        from build123d import Cylinder, Locations, Mode
    except ImportError as error:
        raise RuntimeError(
            "bearing_seat(create_geometry=True) requires build123d. "
            "Install build123d or pass create_geometry=False.",
        ) from error

    with Locations(seat_center):
        Cylinder(
            radius=seat_diameter_mm / 2.0,
            height=seat_depth_mm,
            rotation=(0, 90, 0),
            mode=Mode.SUBTRACT,
        )


__all__ = [
    "BurrDesignData",
    "DESIGN_DATA_FILE",
    "DESIGN_DATA_SCHEMA",
    "bearing_seat",
    "counterbore",
    "heat_set_insert_pocket",
    "__version__",
    "m3_clearance_hole",
    "standoff_boss",
    "straight_slot",
]
