"""Check analytic carriers and all retained triangles against per-face OCCT."""
import json, pathlib, sys
import numpy as np
import cadquery as cq
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRep import BRep_Tool
from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
from OCP.TopAbs import TopAbs_REVERSED
from OCP.gp import gp_Pnt, gp_Vec
root = pathlib.Path(sys.argv[1])
reference = json.loads((root / 'reference.json').read_text())
look = json.loads((root / 'look.json').read_text())
assert len(reference) == len(look)

def vector(p):
    return np.array([p[k] for k in ('x', 'y', 'z')]) if isinstance(p, dict) else np.array(p)
rows = []
for exact, actual in zip(reference, look):
    assert (exact['model'], exact['face']) == (actual['model'], actual['face'])
    assert len(exact['samples']) == len(actual['samples']) and len(exact['boundary']) == len(actual['boundary'])
    row = {k: exact[k] for k in ('model', 'source_sha256', 'face', 'surface', 'area')}
    row['max_point_error'] = max((s['point_error'] for s in actual['samples']))
    row['max_normal_error'] = max((np.linalg.norm(vector(a['normal']) - vector(e['normal'])) for e, a in zip(exact['samples'], actual['samples'])))
    row['missing_interior_inverse'] = sum((s['inverse_error'] is None for s in actual['samples']))
    row['missing_interior_nearest'] = sum((s['nearest_error'] is None for s in actual['samples']))
    row['max_inverse_error'] = max((s['inverse_error'] or 0 for s in actual['samples']))
    row['max_nearest_error'] = max((s['nearest_error'] or 0 for s in actual['samples']))
    row['missing_boundary_inverse'] = sum((s['inverse_error'] is None for s in actual['boundary']))
    row['missing_boundary_nearest'] = sum((s['nearest_error'] is None for s in actual['boundary']))
    row['max_boundary_inverse_error'] = max((s['inverse_error'] or 0 for s in actual['boundary']))
    row['max_boundary_nearest_error'] = max((s['nearest_error'] or 0 for s in actual['boundary']))
    row['analytic_agrees'] = all((row[k] < 1e-07 for k in ('max_point_error', 'max_normal_error', 'max_inverse_error', 'max_nearest_error', 'max_boundary_inverse_error', 'max_boundary_nearest_error'))) and (not any((row[k] for k in ('missing_interior_inverse', 'missing_interior_nearest', 'missing_boundary_inverse', 'missing_boundary_nearest'))))
    row['face_losses'] = actual['losses']
    row['triangles'] = 0
    if actual['mesh']:
        reader = STEPControl_Reader()
        assert reader.ReadFile(exact['path']) == IFSelect_RetDone
        reader.TransferRoots()
        face = cq.Shape.cast(reader.OneShape()).Faces()[0]
        pos = np.array([vector(p) for p in actual['mesh']['positions']])
        tri = actual['mesh']['triangles']
        row['triangles'] = len(tri)
        row['mesh_area'] = actual['mesh']['area']
        row['relative_area_error'] = abs(row['mesh_area'] / row['area'] - 1)
        row['max_vertex_distance'] = max((cq.Vertex.makeVertex(*p).distance(face) for p in pos))
        row['max_centroid_distance'] = max((cq.Vertex.makeVertex(*pos[t].mean(axis=0)).distance(face) for t in tri))
        native = face.wrapped
        surf = BRep_Tool.Surface_s(native)
        wrong_winding = 0
        min_normal_dot = 1.0
        for indices in tri:
            points = pos[indices]
            center = points.mean(axis=0)
            cross = np.cross(points[1] - points[0], points[2] - points[0])
            length = np.linalg.norm(cross)
            if length < 1e-14:
                continue
            projection = GeomAPI_ProjectPointOnSurf(gp_Pnt(*center), surf, *exact['bounds'], 1e-10)
            assert projection.NbPoints() > 0
            u, v = projection.LowerDistanceParameters()
            point = gp_Pnt()
            du = gp_Vec()
            dv = gp_Vec()
            surf.D1(u, v, point, du, dv)
            normal = du.Crossed(dv)
            normal.Normalize()
            if native.Orientation() == TopAbs_REVERSED:
                normal.Reverse()
            dot = float(np.dot(cross / length, [normal.X(), normal.Y(), normal.Z()]))
            min_normal_dot = min(min_normal_dot, dot)
            wrong_winding += dot <= 0
        row['wrong_winding_triangles'] = wrong_winding
        row['min_triangle_normal_dot'] = min_normal_dot
        row['trim_agrees'] = not wrong_winding and row['relative_area_error'] < 0.01 and (row['max_vertex_distance'] < 0.011) and (row['max_centroid_distance'] < 0.011)
    else:
        row['trim_agrees'] = None
    rows.append(row)
    print(json.dumps(row), flush=True)
(root / 'summary.json').write_text(json.dumps(rows, indent=2) + '\n')
assert all((r['analytic_agrees'] for r in rows))
assert all((r['trim_agrees'] is not False for r in rows))
print('PASS', len(rows), 'carriers;', sum((r['triangles'] > 0 for r in rows)), 'trimmed meshes')
