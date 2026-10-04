"""Compare occurrence pairs using unique placement, source identity and geometry.

Requires the corpus OCCT environment (cadquery, numpy, scipy), an OCCT report,
scene-dump JSON including surface samples, and a Burr report. Never pairs by
name order; ambiguous placements or incompatible source geometry are errors.
"""
import argparse
import json
import pathlib
import re

import cadquery as cq
import numpy as np
from scipy.optimize import linear_sum_assignment

from occt_components import components


def identity(name):
    leaf = name.rsplit('/', 1)[-1]
    if leaf.lower().startswith(('solid', 'compound')):
        return None
    return ''.join(re.findall('[a-z0-9]+', leaf.lower()))


def compare(model, scene_path, burr_path, occt_path, reference_complete=False):
    scene = json.loads(scene_path.read_text(encoding="utf-8"))['parts']
    exact = components(model)
    if len(exact) == 1 and len(cq.Shape.cast(exact[0][1]).Solids()) > 1:
        exact = [('solid:' + str(i), s.wrapped)
                 for i, s in enumerate(cq.Shape.cast(exact[0][1]).Solids())]
    if len(scene) != len(exact):
        raise ValueError('Occurrence count differs; cannot compare pair identities')
    shapes = [cq.Shape.cast(s) for _, s in exact]
    boxes = []
    for shape in shapes:
        b = shape.BoundingBox()
        boxes.append([b.xmin, b.zmin, -b.ymax, b.xmax, b.zmax, -b.ymin])
    x = np.array([p['min'] + p['max'] for p in scene])
    y = np.array(boxes)
    cost = np.linalg.norm(x[:, None] - y[None, :], axis=2)
    rows, cols = linear_sum_assignment(cost)
    mapping = {}
    evidence = []
    for i, j in zip(rows.tolist(), cols.tolist()):
        distance = float(cost[i, j])
        alternative = float(min((cost[i, k] for k in range(len(exact)) if k != j), default=float("inf")))
        # The corpus tessellation can deviate by up to 0.065 mm in these bounds.
        # A 0.1 mm bound and an independent 0.2 mm alternative margin must both
        # hold. These are matching limits, not interference tolerances.
        if distance > 0.1 or alternative - distance < 0.2:
            raise ValueError(f'Ambiguous placement for occurrence {i}: {distance}, {alternative}')
        left_name = scene[i]['name'] or ''
        right_name = exact[j][0]
        left_id, right_id = identity(left_name), identity(right_name)
        if left_id and right_id and left_id != right_id:
            raise ValueError(f'Identity mismatch: {left_name!r}, {right_name!r}')
        samples = scene[i].get('sample_points')
        if not samples:
            raise ValueError('scene-dump must include world-space surface samples')
        sample_error = max(cq.Vertex.makeVertex(p[0], -p[2], p[1]).distance(shapes[j])
                           for p in samples)
        if sample_error > 0.1:
            raise ValueError(f'Geometry mismatch for occurrence {i}: {sample_error}')
        mapping[i] = j
        evidence.append(dict(burr=i, occt=j, burr_name=left_name, occt_name=right_name,
                             box_error_mm=distance, alternative_box_error_mm=alternative if len(exact) > 1 else None,
                             surface_sample_error_mm=sample_error, source_identity=left_id or right_id))
    report = json.loads(burr_path.read_text(encoding="utf-8"))['report']
    reference = json.loads(occt_path.read_text(encoding="utf-8"))
    if not reference.get('pair_check_complete', reference_complete):
        raise ValueError('OCCT reference did not finish its pair scan')
    if reference['names'] != [n for n, _ in exact]:
        raise ValueError('OCCT occurrence identity changed from the recorded reference')
    positives = {tuple(f['pair']): f for f in reference['findings']}
    def pair(f):
        return tuple(sorted(mapping[c['occurrence_index']] for c in f['components']))
    found = {pair(f): f for f in report['findings']}
    unresolved = {pair(f): f for f in report.get('unresolved_pairs', [])}
    return dict(model=model.name, mapping=evidence, exact_pair_count=len(positives),
                confirmed_pair_count=len(found), matched_pairs=sorted(found.keys() & positives.keys()),
                extra_pairs=[dict(pair=p, burr=found[p]) for p in sorted(found.keys() - positives.keys())],
                missing_pairs=[dict(pair=p, occt=positives[p], unresolved=unresolved.get(p))
                               for p in sorted(positives.keys() - found.keys())],
                unresolved_pairs=[dict(pair=p, burr=f, occt_positive=p in positives)
                                  for p, f in sorted(unresolved.items())])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model', type=pathlib.Path)
    parser.add_argument('--scene', type=pathlib.Path, required=True)
    parser.add_argument('--burr', type=pathlib.Path, required=True)
    parser.add_argument('--occt', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--reference-complete', action='store_true', help='Assert a documented full scan for legacy OCCT logs without a completion flag')
    args = parser.parse_args()
    result = compare(args.model, args.scene, args.burr, args.occt, args.reference_complete)
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding="utf-8")
    print(args.model.name, 'matched', len(result['matched_pairs']), 'extra', len(result['extra_pairs']),
          'missing', len(result['missing_pairs']), 'unresolved', len(result['unresolved_pairs']))


if __name__ == '__main__':
    main()
