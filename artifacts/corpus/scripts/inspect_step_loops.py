"""Inspect disconnected STEP bounds without repairing source geometry.
Reductions retain source headers and IDs and must stay local; source licences
are recorded in corpus sources.csv. Requires the OCCT reference Python env.
Run under the shared corpus/build lock. Summary contains IDs and errors only.
"""
import argparse, hashlib, json, pathlib, re, collections, math
import cadquery as cq
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRep import BRep_Tool
from OCP.BRepTools import BRepTools
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
from OCP.gp import gp_Pnt
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--model', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
a = parser.parse_args()
model = a.model
out = a.output
out.mkdir(parents=True, exist_ok=True)
text = model.read_text(encoding='latin1')
data = text.split('DATA;', 1)[1].rsplit('ENDSEC;', 1)[0]
ents = {int(m[1]): m[2].strip() for m in re.finditer("#\\s*(\\d+)\\s*=\\s*((?:[^;']|'(?:[^']|'')*')*);", data, re.S)}
refs = lambda s: [int(x) for x in re.findall('#\\s*(\\d+)', re.sub("'(?:[^']|'')*'", "''", s))]
context = next((i for i, s in ents.items() if 'GEOMETRIC_REPRESENTATION_CONTEXT' in s))
faces = []
for fid, entity in ents.items():
    if not entity.startswith('ADVANCED_FACE('):
        continue
    named = refs(entity)
    for bound in named[:-1]:
        if not re.match('FACE_(?:OUTER_)?BOUND\\s*\\(', ents[bound]):
            continue
        loop = refs(ents[bound])[0]
        if not ents[loop].startswith('EDGE_LOOP('):
            continue
        walks = []
        for use in refs(ents[loop]):
            oriented = ents[use].startswith('ORIENTED_EDGE(')
            edge = refs(ents[use])[-1] if oriented else use
            start, end = refs(ents[edge])[:2]
            if oriented and re.search(',\\s*\\.F\\.\\s*\\)$', ents[use]):
                start, end = (end, start)
            walks.append((start, end, use, edge))
        if walks and all((x[1] == y[0] for x, y in zip(walks, walks[1:] + walks[:1]))):
            continue
        pending = set(range(len(walks)))
        components = []
        while pending:
            first = min(pending)
            pending.remove(first)
            chosen = [first]
            vertices = set(walks[first][:2])
            changed = True
            while changed:
                changed = False
                for i in sorted(pending):
                    if vertices.intersection(walks[i][:2]):
                        chosen.append(i)
                        pending.remove(i)
                        vertices.update(walks[i][:2])
                        changed = True
            ins = collections.Counter((walks[i][1] for i in chosen))
            outs = collections.Counter((walks[i][0] for i in chosen))
            components.append(dict(edge_count=len(chosen), cycle=all((ins[v] == outs[v] == 1 for v in vertices)), balanced=all((ins[v] == outs[v] for v in vertices)), uses=[walks[i][2] for i in chosen]))
        faces.append(dict(face=fid, bound=bound, loop=loop, surface=named[-1], surface_kind=ents[named[-1]].split('(', 1)[0], walks=walks, components=components))
results = []
for f in faces:
    fid = f['face']
    seen = set()
    todo = [fid, context]
    while todo:
        i = todo.pop()
        if i in seen:
            continue
        seen.add(i)
        todo.extend(refs(ents[i]))
    n = max(ents) + 1
    extra = f"#{n}=OPEN_SHELL('',(#{fid}));\n#{n + 1}=SHELL_BASED_SURFACE_MODEL('',(#{n}));\n#{n + 2}=MANIFOLD_SURFACE_SHAPE_REPRESENTATION('',(#{n + 1}),#{context});\n#{n + 3}=APPLICATION_CONTEXT('automotive_design');\n#{n + 4}=PRODUCT_CONTEXT('',#{n + 3},'mechanical');\n#{n + 5}=PRODUCT('face','face','',(#{n + 4}));\n#{n + 6}=PRODUCT_DEFINITION_FORMATION('','',#{n + 5});\n#{n + 7}=PRODUCT_DEFINITION_CONTEXT('part definition',#{n + 3},'design');\n#{n + 8}=PRODUCT_DEFINITION('design','',#{n + 6},#{n + 7});\n#{n + 9}=PRODUCT_DEFINITION_SHAPE('','',#{n + 8});\n#{n + 10}=SHAPE_DEFINITION_REPRESENTATION(#{n + 9},#{n + 2});\n"
    path = out / f'face-{fid}.step'
    path.write_text(text.split('DATA;', 1)[0] + 'DATA;\n' + '\n'.join((f'#{i}={ents[i]};' for i in sorted(seen))) + '\n' + extra + 'ENDSEC;\nEND-ISO-10303-21;\n', encoding='latin1')
    r = STEPControl_Reader()
    assert r.ReadFile(str(path)) == IFSelect_RetDone
    r.TransferRoots()
    sh = cq.Shape.cast(r.OneShape())
    fs = sh.Faces()
    row = dict(face=fid, balanced=all((c['balanced'] for c in f['components'])), surface_kind=f['surface_kind'], face_count=len(fs), source_components=f['components'])
    if len(fs) == 1:
        face = fs[0]
        s = BRep_Tool.Surface_s(face.wrapped)
        row.update(valid=BRepCheck_Analyzer(face.wrapped).IsValid(), area=face.Area(), wire_edges=[len(w.Edges()) for w in face.Wires()], bounds=list(BRepTools.UVBounds_s(face.wrapped)))
        vertices = []
        for vertex in sorted({v for w in f['walks'] for v in w[:2]}):
            point = ents[refs(ents[vertex])[0]]
            xyz = [float(x) for x in re.search(',\\s*\\(([^)]+)\\)\\s*\\)$', point)[1].split(',')]
            project = GeomAPI_ProjectPointOnSurf(gp_Pnt(*xyz), s)
            vertices.append(dict(vertex=vertex, distance=project.LowerDistance()))
        distances = {v['vertex']: v['distance'] for v in vertices}
        row['max_source_vertex_surface_distance'] = max(distances.values())
        for component in row['source_components']:
            component['max_source_vertex_surface_distance'] = max((distances[v] for w in f['walks'] if w[2] in component['uses'] for v in w[:2]))
            component['source_edges'] = sorted({w[3] for w in f['walks'] if w[2] in component['uses']})
        degree = collections.Counter((v for w in f['walks'] for v in w[:2]))
        odd = [v for v, d in degree.items() if d % 2]

        def position(v):
            point = ents[refs(ents[v])[0]]
            return tuple((float(x) for x in re.search(',\\s*\\(([^)]+)\\)\\s*\\)$', point)[1].split(',')))
        row['odd_endpoint_count'] = len(odd)
        row['nearest_odd_endpoint_distances'] = [min((math.dist(position(v), position(w)) for w in odd if w != v)) for v in odd]
        row['bound'] = f['bound']
        row['loop'] = f['loop']
        row['surface'] = f['surface']
    results.append(row)
    print(json.dumps(row), flush=True)
summary = dict(source=model.name, source_sha256=hashlib.sha256(model.read_bytes()).hexdigest(), occt_version=OCP.__version__, note='Original source vertex positions projected onto each OCCT carrier; standard OCCT STEP transfer defaults retained; no application-level wire repair, snapping, or inferred replacement edge. Local reductions retain source headers and IDs.', faces=results)
(out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
