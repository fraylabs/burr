from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.GeomAbs import GeomAbs_Cylinder, GeomAbs_Plane
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS


SCHEMA_VERSION = "burr.ocp-step-cylinders.v1"


def _round_vector(values: tuple[float, float, float]) -> list[float]:
    return [round(float(value), 9) for value in values]


def extract_step_cylinders(path: str | Path) -> dict[str, Any]:
    step_path = Path(path)
    reader = STEPControl_Reader()
    status = reader.ReadFile(str(step_path))
    if status != IFSelect_RetDone:
        raise RuntimeError(f"OpenCascade could not read STEP file: {step_path}")

    reader.TransferRoots()
    shape = reader.OneShape()
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    cylinders: list[dict[str, Any]] = []
    planes: list[dict[str, Any]] = []

    while explorer.More():
        face = TopoDS.Face_s(explorer.Current())
        surface = BRepAdaptor_Surface(face, True)
        if surface.GetType() == GeomAbs_Cylinder:
            cylinder = surface.Cylinder()
            location = cylinder.Location()
            direction = cylinder.Axis().Direction()
            cylinders.append(
                {
                    "point_mm": _round_vector((location.X(), location.Y(), location.Z())),
                    "axis": _round_vector((direction.X(), direction.Y(), direction.Z())),
                    "radius_mm": round(float(cylinder.Radius()), 9),
                },
            )
        elif surface.GetType() == GeomAbs_Plane:
            plane = surface.Plane()
            location = plane.Location()
            direction = plane.Axis().Direction()
            planes.append(
                {
                    "point_mm": _round_vector((location.X(), location.Y(), location.Z())),
                    "normal": _round_vector((direction.X(), direction.Y(), direction.Z())),
                },
            )
        explorer.Next()

    return {
        "schema_version": SCHEMA_VERSION,
        "units": "mm",
        "cylinders": cylinders,
        "planes": planes,
        "warnings": [],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Extract cylindrical STEP faces as Burr JSON.")
    parser.add_argument("step", help="STEP/STP file to inspect")
    args = parser.parse_args(argv)

    try:
        data = extract_step_cylinders(args.step)
    except Exception as error:
        print(str(error), file=sys.stderr)
        return 2

    print(json.dumps(data, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
