"""Reduce each signed-major STEP face locally and query its OCCT reference.
No source CAD is intended for version control. Source licences are in Burr's
corpus sources.csv; all reductions retain their source header and entity ids.
"""
import argparse, hashlib, json, pathlib, re
import cadquery as cq
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRep import BRep_Tool
from OCP.BRepTools import BRepTools
from OCP.BRepClass import BRepClass_FaceClassifier
from OCP.TopAbs import TopAbs_IN, TopAbs_REVERSED
from OCP.gp import gp_Pnt, gp_Pnt2d, gp_Vec
p = argparse.ArgumentParser()
p.add_argument('--corpus', type=pathlib.Path, required=True)
p.add_argument('--output', type=pathlib.Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=True)
refs = lambda s: [int(x) for x in re.findall('#\\s*(\\d+)', re.sub("'(?:[^']|'')*'", "''", s))]
manifest = []
for model in sorted((a.corpus / 'models').glob('openamr*.STEP')):
    text = model.read_text(encoding='latin1')
    data = text.split('DATA;', 1)[1].rsplit('ENDSEC;', 1)[0]
    ents = {int(m[1]): m[2].strip() for m in re.finditer("#\\s*(\\d+)\\s*=\\s*((?:[^;']|'(?:[^']|'')*')*);", data, re.S)}
    negative = {i for i, s in ents.items() if re.match('TOROIDAL_SURFACE\\s*\\(', s) and float(re.search(',\\s*([-+0-9.Ee]+)\\s*,\\s*[-+0-9.Ee]+\\s*\\)\\s*$', s)[1]) < 0}
    contexts = [i for i, s in ents.items() if 'GEOMETRIC_REPRESENTATION_CONTEXT' in s]
    context = contexts[0]
    for face, s in sorted(ents.items()):
        if not re.match('ADVANCED_FACE\\s*\\(', s) or refs(s)[-1] not in negative:
            continue
        sid = refs(s)[-1]
        seen = set()
        todo = [face, context]
        while todo:
            i = todo.pop()
            if i in seen:
                continue
            seen.add(i)
            todo.extend(refs(ents[i]))
        shell = max(ents) + 1
        extra = f"#{shell}=OPEN_SHELL('',(#{face}));\n#{shell + 1}=SHELL_BASED_SURFACE_MODEL('',(#{shell}));\n#{shell + 2}=MANIFOLD_SURFACE_SHAPE_REPRESENTATION('',(#{shell + 1}),#{context});\n#{shell + 3}=APPLICATION_CONTEXT('automotive_design');\n#{shell + 4}=PRODUCT_CONTEXT('',#{shell + 3},'mechanical');\n#{shell + 5}=PRODUCT('face','face','', (#{shell + 4}));\n#{shell + 6}=PRODUCT_DEFINITION_FORMATION('','',#{shell + 5});\n#{shell + 7}=PRODUCT_DEFINITION_CONTEXT('part definition',#{shell + 3},'design');\n#{shell + 8}=PRODUCT_DEFINITION('design','',#{shell + 6},#{shell + 7});\n#{shell + 9}=PRODUCT_DEFINITION_SHAPE('','',#{shell + 8});\n#{shell + 10}=SHAPE_DEFINITION_REPRESENTATION(#{shell + 9},#{shell + 2});\n"
        reduced = a.output / (model.stem + '-face-' + str(face) + '.step')
        reduced.write_text(text.split('DATA;', 1)[0] + 'DATA;\n' + '\n'.join((f'#{i}={ents[i]};' for i in sorted(seen))) + '\n' + extra + 'ENDSEC;\nEND-ISO-10303-21;\n')
        reader = STEPControl_Reader()
        assert reader.ReadFile(str(reduced)) == IFSelect_RetDone
        reader.TransferRoots()
        faces = cq.Shape.cast(reader.OneShape()).Faces()
        assert len(faces) == 1, (reduced, len(faces))
        exact = faces[0]
        native = exact.wrapped
        surf = BRep_Tool.Surface_s(native)
        u0, u1, v0, v1 = BRepTools.UVBounds_s(native)
        samples = []
        for i in range(1, 24):
            for j in range(1, 24):
                u = u0 + (u1 - u0) * i / 24
                v = v0 + (v1 - v0) * j / 24
                if BRepClass_FaceClassifier(native, gp_Pnt2d(u, v), 1e-09).State() != TopAbs_IN:
                    continue
                point = gp_Pnt()
                du = gp_Vec()
                dv = gp_Vec()
                surf.D1(u, v, point, du, dv)
                normal = du.Crossed(dv)
                if normal.Magnitude() < 1e-09:
                    continue
                normal.Normalize()
                if native.Orientation() == TopAbs_REVERSED:
                    normal.Reverse()
                samples.append(dict(uv=[u, v], point=[point.X(), point.Y(), point.Z()], normal=[normal.X(), normal.Y(), normal.Z()]))
        assert samples, (reduced, (u0, u1, v0, v1))
        boundary = []
        for edge in exact.Edges():
            for t in [0.0, 0.25, 0.5, 0.75, 1.0]:
                point = edge.positionAt(t)
                boundary.append([point.x, point.y, point.z])
        row = dict(model=model.name, source_sha256=hashlib.sha256(model.read_bytes()).hexdigest(), face=face, surface=sid, shell=shell, path=str(reduced.resolve()), same_sense=s.rstrip().endswith('.T.)'), occt_reversed=native.Orientation() == TopAbs_REVERSED, area=exact.Area(), bounds=[u0, u1, v0, v1], samples=samples, boundary=boundary)
        row['same_sense'] = bool(re.search(',\\s*\\.T\\.\\s*\\)\\s*$', s))
        manifest.append(row)
        print(model.name, face, 'area', row['area'], 'samples', len(samples), flush=True)
(a.output / 'reference.json').write_text(json.dumps(manifest) + '\n')
print('TOTAL', len(manifest), flush=True)
