from __future__ import annotations

import argparse
import math
import struct
import zlib
from pathlib import Path

from OCP.BRep import BRep_Tool
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS


Vec3 = tuple[float, float, float]
Color = tuple[int, int, int]


def main() -> int:
    parser = argparse.ArgumentParser(description="Render a simple PNG preview from a STEP file.")
    parser.add_argument("step", help="STEP file to render")
    parser.add_argument("png", help="PNG output path")
    parser.add_argument("--width", type=int, default=960)
    parser.add_argument("--height", type=int, default=720)
    parser.add_argument("--title", default="")
    args = parser.parse_args()

    triangles = load_step_triangles(Path(args.step))
    if not triangles:
        raise RuntimeError(f"No mesh triangles found in {args.step}")

    image = render_triangles(triangles, args.width, args.height, args.title)
    write_png(Path(args.png), args.width, args.height, image)
    return 0


def load_step_triangles(path: Path) -> list[tuple[Vec3, Vec3, Vec3]]:
    reader = STEPControl_Reader()
    status = reader.ReadFile(str(path))
    if status != IFSelect_RetDone:
        raise RuntimeError(f"OpenCascade could not read STEP file: {path}")

    reader.TransferRoots()
    shape = reader.OneShape()
    BRepMesh_IncrementalMesh(shape, 0.45)

    triangles: list[tuple[Vec3, Vec3, Vec3]] = []
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    while explorer.More():
        face = TopoDS.Face_s(explorer.Current())
        location = face.Location()
        triangulation = BRep_Tool.Triangulation_s(face, location)
        if triangulation is None:
            explorer.Next()
            continue

        transform = location.Transformation()
        nodes: dict[int, Vec3] = {}
        for node_index in range(1, triangulation.NbNodes() + 1):
            point = triangulation.Node(node_index).Transformed(transform)
            nodes[node_index] = (point.X(), point.Y(), point.Z())

        for triangle_index in range(1, triangulation.NbTriangles() + 1):
            a, b, c = triangulation.Triangle(triangle_index).Get()
            triangles.append((nodes[a], nodes[b], nodes[c]))

        explorer.Next()

    return triangles


def render_triangles(
    triangles: list[tuple[Vec3, Vec3, Vec3]],
    width: int,
    height: int,
    title: str,
) -> bytearray:
    background = (248, 250, 252)
    pixels = bytearray(background * (width * height))
    z_buffer = [-float("inf")] * (width * height)

    points = [point for triangle in triangles for point in triangle]
    center = (
        sum(point[0] for point in points) / len(points),
        sum(point[1] for point in points) / len(points),
        sum(point[2] for point in points) / len(points),
    )
    view_dir = normalize((-0.72, 0.58, -0.38))
    world_up = (0.0, 0.0, 1.0)
    right = normalize(cross(view_dir, world_up))
    up = normalize(cross(right, view_dir))

    projected = [(dot(sub(point, center), right), dot(sub(point, center), up)) for point in points]
    min_x = min(point[0] for point in projected)
    max_x = max(point[0] for point in projected)
    min_y = min(point[1] for point in projected)
    max_y = max(point[1] for point in projected)
    span_x = max(max_x - min_x, 1.0)
    span_y = max(max_y - min_y, 1.0)
    padding = 72
    title_space = 58 if title else 24
    scale = min((width - padding * 2) / span_x, (height - padding - title_space) / span_y)
    offset_x = width / 2 - ((min_x + max_x) / 2) * scale
    offset_y = (height + title_space) / 2 + ((min_y + max_y) / 2) * scale

    light = normalize((-0.4, -0.6, 1.0))
    base = (84, 125, 166)
    screen_triangles = []
    for triangle in triangles:
        normal = normalize(cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0])))
        shade = 0.58 + 0.42 * abs(dot(normal, light))
        color = tuple(max(0, min(255, int(channel * shade))) for channel in base)
        screen = []
        for point in triangle:
            rel = sub(point, center)
            x = dot(rel, right)
            y = dot(rel, up)
            z = dot(rel, view_dir)
            screen.append((offset_x + x * scale, offset_y - y * scale, z))
        screen_triangles.append((sum(point[2] for point in screen) / 3.0, screen, color))

    for _depth, screen, color in sorted(screen_triangles, key=lambda item: item[0]):
        rasterize_triangle(pixels, z_buffer, width, height, screen, color)

    if title:
        draw_text(pixels, width, height, 28, 24, title, (15, 23, 42), scale=3)

    return pixels


def rasterize_triangle(
    pixels: bytearray,
    z_buffer: list[float],
    width: int,
    height: int,
    triangle: list[tuple[float, float, float]],
    color: Color,
) -> None:
    xs = [point[0] for point in triangle]
    ys = [point[1] for point in triangle]
    min_x = max(0, int(math.floor(min(xs))))
    max_x = min(width - 1, int(math.ceil(max(xs))))
    min_y = max(0, int(math.floor(min(ys))))
    max_y = min(height - 1, int(math.ceil(max(ys))))
    area = edge_function(triangle[0], triangle[1], triangle[2])
    if abs(area) < 1e-9:
        return

    for y in range(min_y, max_y + 1):
        for x in range(min_x, max_x + 1):
            sample = (x + 0.5, y + 0.5, 0.0)
            w0 = edge_function(triangle[1], triangle[2], sample) / area
            w1 = edge_function(triangle[2], triangle[0], sample) / area
            w2 = edge_function(triangle[0], triangle[1], sample) / area
            if w0 < -1e-6 or w1 < -1e-6 or w2 < -1e-6:
                continue
            z = w0 * triangle[0][2] + w1 * triangle[1][2] + w2 * triangle[2][2]
            index = y * width + x
            if z < z_buffer[index]:
                continue
            z_buffer[index] = z
            pixel_index = index * 3
            pixels[pixel_index : pixel_index + 3] = bytes(color)


def edge_function(a: tuple[float, float, float], b: tuple[float, float, float], c: tuple[float, float, float]) -> float:
    return (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])


FONT = {
    "A": ["111", "101", "111", "101", "101"],
    "B": ["110", "101", "110", "101", "110"],
    "C": ["111", "100", "100", "100", "111"],
    "D": ["110", "101", "101", "101", "110"],
    "E": ["111", "100", "110", "100", "111"],
    "F": ["111", "100", "110", "100", "100"],
    "G": ["111", "100", "101", "101", "111"],
    "H": ["101", "101", "111", "101", "101"],
    "I": ["111", "010", "010", "010", "111"],
    "J": ["001", "001", "001", "101", "111"],
    "K": ["101", "101", "110", "101", "101"],
    "L": ["100", "100", "100", "100", "111"],
    "M": ["101", "111", "111", "101", "101"],
    "N": ["101", "111", "111", "111", "101"],
    "O": ["111", "101", "101", "101", "111"],
    "P": ["111", "101", "111", "100", "100"],
    "Q": ["111", "101", "101", "111", "001"],
    "R": ["111", "101", "111", "110", "101"],
    "S": ["111", "100", "111", "001", "111"],
    "T": ["111", "010", "010", "010", "010"],
    "U": ["101", "101", "101", "101", "111"],
    "V": ["101", "101", "101", "101", "010"],
    "W": ["101", "101", "111", "111", "101"],
    "X": ["101", "101", "010", "101", "101"],
    "Y": ["101", "101", "010", "010", "010"],
    "Z": ["111", "001", "010", "100", "111"],
    "0": ["111", "101", "101", "101", "111"],
    "1": ["010", "110", "010", "010", "111"],
    "2": ["111", "001", "111", "100", "111"],
    "3": ["111", "001", "111", "001", "111"],
    "4": ["101", "101", "111", "001", "001"],
    "5": ["111", "100", "111", "001", "111"],
    "6": ["111", "100", "111", "101", "111"],
    "7": ["111", "001", "001", "001", "001"],
    "8": ["111", "101", "111", "101", "111"],
    "9": ["111", "101", "111", "001", "111"],
    "-": ["000", "000", "111", "000", "000"],
    " ": ["000", "000", "000", "000", "000"],
}


def draw_text(
    pixels: bytearray,
    width: int,
    height: int,
    x: int,
    y: int,
    text: str,
    color: Color,
    scale: int = 2,
) -> None:
    cursor = x
    for char in text.upper():
        glyph = FONT.get(char, FONT[" "])
        for row_index, row in enumerate(glyph):
            for col_index, value in enumerate(row):
                if value != "1":
                    continue
                for dy in range(scale):
                    for dx in range(scale):
                        px = cursor + col_index * scale + dx
                        py = y + row_index * scale + dy
                        if 0 <= px < width and 0 <= py < height:
                            index = (py * width + px) * 3
                            pixels[index : index + 3] = bytes(color)
        cursor += 4 * scale


def write_png(path: Path, width: int, height: int, pixels: bytearray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = bytearray()
    row_bytes = width * 3
    for y in range(height):
        raw.append(0)
        start = y * row_bytes
        raw.extend(pixels[start : start + row_bytes])

    def chunk(kind: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), level=9))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(png)


def sub(a: Vec3, b: Vec3) -> Vec3:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a: Vec3, b: Vec3) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a: Vec3, b: Vec3) -> Vec3:
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def normalize(vector: Vec3) -> Vec3:
    length = math.sqrt(dot(vector, vector))
    if length <= 1e-12:
        return (0.0, 0.0, 0.0)
    return (vector[0] / length, vector[1] / length, vector[2] / length)


if __name__ == "__main__":
    raise SystemExit(main())
