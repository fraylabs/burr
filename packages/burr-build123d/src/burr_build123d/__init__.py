"""Small build123d helpers for emitting Burr design data.

This package does not replace build123d. It records mechanical intent while a
normal build123d design file creates geometry.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


DESIGN_DATA_FILE = "burr-design-data.json"
DESIGN_DATA_SCHEMA = "burr.design-data.v1"
__version__ = "0.5.0"


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

    def source(self, path: str, kind: str = "design_file") -> None:
        self.sources.append({"kind": kind, "path": path})

    def artifact(self, path: str, kind: str = "step") -> None:
        self.artifacts.append({"kind": kind, "path": path})

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
    ) -> None:
        if not intent:
            raise ValueError("clearance_hole intent must be a non-empty string.")
        self.features.append(
            {
                "id": feature_id,
                "part": part,
                "kind": "clearance_hole",
                "intent": intent,
                "fastener": fastener,
                "diameter_mm": float(diameter_mm),
                "center_mm": _round_vector(center),
                "axis": _round_vector(axis),
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


__all__ = [
    "BurrDesignData",
    "DESIGN_DATA_FILE",
    "DESIGN_DATA_SCHEMA",
    "__version__",
    "m3_clearance_hole",
]
