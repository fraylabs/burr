"""Generate the small STEP assemblies used by Burr's intersection proof.

The checked-in STEP files are the test inputs. This script records how they
were produced so a dependency upgrade can regenerate them deliberately.
"""

from pathlib import Path

from build123d import Box, Compound, export_step


OUTPUT = Path(__file__).parent.parent / "tests" / "fixtures" / "intersections"
TIMESTAMP = "2026-01-01T00:00:00"


def box(label: str, size: tuple[float, float, float], x: float = 0.0) -> Box:
    shape = Box(*size)
    shape.label = label
    shape.position = (x, 0.0, 0.0)
    return shape


def assembly(name: str, first: Box, second: Box) -> None:
    model = Compound(label=name, children=[first, second])
    output = OUTPUT / f"{name}.step"
    export_step(model, output, timestamp=TIMESTAMP)
    normalize_step(output)


def normalize_step(path: Path) -> None:
    """Remove exporter-only trailing spaces so fixtures stay diff-clean."""
    lines = path.read_text(encoding="utf-8").splitlines()
    path.write_text("\n".join(line.rstrip() for line in lines) + "\n", encoding="utf-8")


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    assembly(
        "separated",
        box("fixed", (10.0, 10.0, 10.0)),
        box("moving", (10.0, 10.0, 10.0), x=15.0),
    )
    assembly(
        "touching",
        box("fixed", (10.0, 10.0, 10.0)),
        box("moving", (10.0, 10.0, 10.0), x=10.0),
    )
    assembly(
        "intersecting",
        box("fixed", (10.0, 10.0, 10.0)),
        box("moving", (10.0, 10.0, 10.0), x=8.0),
    )
    assembly(
        "contained",
        box("outer", (20.0, 20.0, 20.0)),
        box("inner", (4.0, 4.0, 4.0)),
    )
if __name__ == "__main__":
    main()
