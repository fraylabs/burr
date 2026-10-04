"""Gate combined import changes against a freshly measured Burr baseline.

Run under the shared build lock in the documented OCCT environment. Bench
outputs come from run_integrated.py; detailed output stays local. No matching
or Boolean tolerances are changed. Pair-limited references certify reported
pairs only, and cannot certify the rest of an assembly.
"""
import argparse
from collections import Counter
import hashlib
import json
import pathlib
import subprocess

from compare_pairs import compare


def read(path):
    return json.loads(path.read_text())


def measured(directory, name):
    metrics = read(directory / (name + '.burr.metrics.json'))
    if metrics['timeout']:
        result = dict(metrics.get('load_metadata', {}))
        result['report'] = dict(outcome='incomplete', findings=[], unresolved_pairs=[],
                              incomplete_reasons=[dict(code='measurement_cap',
                                                       message=f"{metrics['timeout_limit_s']:g} s {metrics['cap']} cap; {metrics.get('cap_reason', 'measurement incomplete')}; no completed Burr check")])
        return result
    return read(directory / (name + '.burr.json'))


def pair(finding):
    return tuple(sorted(c['occurrence_index'] for c in finding['components']))


def diagnostic_reason(record):
    if 'terminal_reason' in record:
        return record['terminal_reason']
    if 'conversion_failure_kind' in record:
        detail = record.get('refusal_tag') or record['conversion_failure_kind']
        return f"conversion:{record['conversion_stage']}:{detail}"
    raise ValueError(f'Unrecognized face diagnostic schema: {record}')


def facts(result):
    report = result.get('report', {})
    return dict(parts=result.get('parts'), faces=result.get('step_import'),
                triangles=result.get('triangles'), outcome=report.get('outcome', 'load error'),
                confirmed=sorted(pair(f) for f in report.get('findings', [])),
                unresolved=sorted((pair(f), f['code']) for f in report.get('unresolved_pairs', [])),
                reasons=report.get('incomplete_reasons', []),
                structure_errors=result.get('structure_errors', []), error=result.get('error'))


def bowden_reference(model, scene, burr, output, tracked):
    """Fresh OCCT Common for the previous 18 pairs plus any new findings."""
    import cadquery as cq
    from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
    from OCP.BRepCheck import BRepCheck_Analyzer
    from occt_components import components
    exact = components(model)
    reference = dict(names=[n for n, _ in exact], findings=[], pair_check_complete=True)
    mapping_path = output / 'bowden-mapping-reference.json'
    mapping_path.write_text(json.dumps(reference))
    matched = compare(model, scene, burr, mapping_path, reported_pairs_only=True)
    mapping = {r['burr']: r['occt'] for r in matched['mapping']}
    previous = read(tracked / 'conforming-mesh/bowden-reported-pairs.json')['pairs']
    checked = {tuple(sorted(p['occt_pair'])) for p in previous}
    checked.update(tuple(p['pair']) for p in matched['extra_pairs'])
    # Prior unresolved contacts use the stable original OCCT identity from the
    # recorded reference. Current confirmed occurrences are freshly rematched.
    for old in previous:
        for i, j in zip(old['burr_pair'], old['occt_pair']):
            if i in mapping and mapping[i] != j:
                raise RuntimeError('Bowden occurrence identity changed')
    details = []
    shapes = [cq.Shape.cast(s) for _, s in exact]
    for i, j in sorted(checked):
        valid = [bool(BRepCheck_Analyzer(shapes[k].wrapped).IsValid()) for k in (i, j)]
        common = BRepAlgoAPI_Common(shapes[i].wrapped, shapes[j].wrapped)
        common.Build()
        if not all(valid) or not common.IsDone() or not BRepCheck_Analyzer(common.Shape()).IsValid():
            raise RuntimeError(f'Bowden Common is unverified: {i}:{j}')
        volume = abs(cq.Shape.cast(common.Shape()).Volume())
        threshold = max(1e-6, min(abs(shapes[i].Volume()), abs(shapes[j].Volume())) * 1e-9)
        entry = dict(pair=[i, j], volume_mm3=volume, threshold_mm3=threshold, valid=valid,
                     common_valid=True, positive=volume > threshold)
        details.append(entry)
        if volume > threshold:
            reference['findings'].append(entry)
    reference.update(pair_check_scope='reported_pairs', checked_pairs=sorted(checked), checks=details)
    path = output / 'bowden-occt.json'
    path.write_text(json.dumps(reference, indent=2) + '\n')
    return path


def two_part_reference(model, output):
    """Complete exact pair proof for the two assembly-decoder repros."""
    import cadquery as cq
    from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
    from OCP.BRepCheck import BRepCheck_Analyzer
    from occt_components import components
    exact = components(model)
    if len(exact) != 2:
        raise RuntimeError(f'Repro must expose two exact occurrences: {model.name}')
    shapes = [cq.Shape.cast(s) for _, s in exact]
    if not all(BRepCheck_Analyzer(s.wrapped).IsValid() for s in shapes):
        raise RuntimeError(f'Invalid exact repro solid: {model.name}')
    common = BRepAlgoAPI_Common(shapes[0].wrapped, shapes[1].wrapped)
    common.Build()
    if not common.IsDone() or not BRepCheck_Analyzer(common.Shape()).IsValid():
        raise RuntimeError(f'Unverified exact repro Common: {model.name}')
    volume = abs(cq.Shape.cast(common.Shape()).Volume())
    threshold = max(1e-6, min(abs(s.Volume()) for s in shapes) * 1e-9)
    findings = [dict(pair=[0, 1], volume_mm3=volume)] if volume > threshold else []
    path = output / (model.name + '.occt.json')
    path.write_text(json.dumps(dict(names=[n for n, _ in exact], findings=findings,
                                   pair_check_complete=True, pair_check_scope='all_pairs',
                                   common_volume_mm3=volume, threshold_mm3=threshold), indent=2) + '\n')
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ('corpus', 'reference-logs', 'before', 'after', 'scene-binary', 'output'):
        parser.add_argument('--' + option, type=pathlib.Path, required=True)
    parser.add_argument('--current-main', type=pathlib.Path)
    parser.add_argument("--require-switchwire-complete", action="store_true",
                        help="Reject a capped Switchwire load after the closure-axis fix")
    args = parser.parse_args()
    tracked = pathlib.Path(__file__).resolve().parent.parent
    args.output.mkdir(parents=True, exist_ok=True)
    protected = {r['model']: r for r in read(tracked / 'conforming-mesh/pair-summary.json')}
    corpus = read(tracked / 'baseline-0.35.json')
    rows = []
    for source in corpus:
        name = source['name']
        old = measured(args.before, name)
        new = measured(args.after, name)
        a, b = facts(old), facts(new)
        current = facts(measured(args.current_main, name)) if args.current_main else None
        if args.require_switchwire_complete and name.startswith('Voron-Switchwire__'):
            for label, directory in [('combined', args.after), ('current main', args.current_main)]:
                if directory is None or read(directory / (name + '.burr.metrics.json'))['timeout']:
                    raise RuntimeError(f'Switchwire did not complete on the closure-axis {label}')
        changed = a != b or (current is not None and current != b)
        outcome = b['outcome']
        if outcome in ('pass', 'fail') and source['occt_verdict'] in ('pass', 'fail') and outcome != source['occt_verdict']:
            raise RuntimeError(f'Wrong conclusive verdict: {name}')
        if outcome == 'pass' and source['occt_verdict'] != 'pass':
            raise RuntimeError(f'Pass has no completed exact clearance reference: {name}')
        previous_pairs = set(a['confirmed']) | (set(current['confirmed']) if current else set())
        removed = previous_pairs - set(b['confirmed'])
        if removed:
            raise RuntimeError(f'Confirmed pair disappeared: {name}: {removed}')
        row = dict(model=name, source_sha256=source['sha256'], changed=changed,
                   before_parts=a['parts'], after_parts=b['parts'],
                   before_lost_faces=a['faces']['lost_faces'] if a['faces'] else None,
                   after_lost_faces=b['faces']['lost_faces'] if b['faces'] else None,
                   before_outcome=a['outcome'], after_outcome=outcome, occt_outcome=source['occt_verdict'],
                   before_measurement_capped=any(r['code'] == 'measurement_cap' for r in a['reasons']),
                   after_measurement_capped=any(r['code'] == 'measurement_cap' for r in b['reasons']),
                   before_confirmed=len(a['confirmed']), after_confirmed=len(b['confirmed']),
                   before_unresolved=len(a['unresolved']), unresolved=len(b['unresolved']),
                   unresolved_codes=dict(Counter(code for _, code in b['unresolved'])),
                   before_triangles=a['triangles'], after_triangles=b['triangles'],
                   incomplete_reasons=b['reasons'],
                   structure_errors=b['structure_errors'])
        if current is not None:
            row['current_main'] = current
        diagnostics = args.after / (name + '.diagnostics.jsonl')
        if diagnostics.exists() and not row['after_measurement_capped']:
            row['face_refusal_reasons'] = dict(Counter(
                diagnostic_reason(json.loads(line))
                for line in diagnostics.read_text().splitlines()))
        if changed or name in protected:
            if any(r['code'] == 'measurement_cap' for r in b['reasons']):
                row.update(comparison_mode='refused', confirmed_true=0, confirmed_false=0,
                           unverified_reason='Measurement capped; no completed scene or pair report')
                rows.append(row)
                print(name, 'incomplete: measurement cap', flush=True)
                continue
            model = args.corpus / source['file']
            scene = args.output / (name + '.scene.json')
            with scene.open('w') as out, scene.with_suffix('.stderr').open('w') as err:
                subprocess.run([str(args.scene_binary.resolve()), str(model)], stdout=out, stderr=err, check=True)
            reference = args.reference_logs / (name + '.occt.json')
            if 'MiniSB_Bowden' in name:
                reference = bowden_reference(model, scene, args.after / (name + '.burr.json'), args.output, tracked)
                modes = [True]
            else:
                modes = [False, True]
            refusals = []
            for limited in modes:
                try:
                    exact = compare(model, scene, args.after / (name + '.burr.json'), reference,
                                    source['occt_pair_scan_complete'], limited)
                except (ValueError, KeyError, json.JSONDecodeError) as error:
                    refusals.append(str(error))
                    continue
                (args.output / (name + '.comparison.json')).write_text(json.dumps(exact, indent=2) + '\n')
                row.update(confirmed_true=len(exact['matched_pairs']), confirmed_false=len(exact['extra_pairs']),
                           comparison_mode=exact['comparison_mode'], reference_scope=exact['reference_scope'],
                           occurrence_mapping_complete=exact['occurrence_mapping_complete'],
                           occt_positives_unconfirmed=len(exact['missing_pairs']),
                           max_box_error_mm=max((r['box_error_mm'] for r in exact['mapping']), default=None),
                           max_surface_error_mm=max((r['surface_sample_error_mm'] for r in exact['mapping']), default=None))
                if row['confirmed_false']:
                    raise RuntimeError(f'False confirmed pairs: {name}: {exact["extra_pairs"]}')
                break
            else:
                if b['confirmed'] or outcome != 'incomplete':
                    raise RuntimeError(f'Unverified conclusive evidence: {name}: {refusals}')
                row.update(confirmed_true=0, confirmed_false=0, comparison_mode='refused',
                           unverified_reason='; '.join(refusals))
            if refusals:
                row['full_comparison_refusal'] = refusals[0]
        if outcome == 'fail' and source['occt_verdict'] not in ('pass', 'fail') and not row.get('confirmed_true'):
            raise RuntimeError(f'Fail has no verified exact overlap: {name}')
        rows.append(row)
        print(name, a['outcome'], '->', outcome, 'pairs', row.get('confirmed_true'), '/', row.get('confirmed_false'), flush=True)
    checked = {r['model']: r for r in rows}
    for name, expected in protected.items():
        if checked[name]['confirmed_true'] < expected['confirmed_true']:
            raise RuntimeError(f'Protected pair regression: {name}')
    center = next(r for r in rows if 'Center_bracket' in r['model'])
    # PR #45's eight M8/bracket pairs are protected by occurrence identity,
    # rather than an exact count that would reject newly verified M5 pairs.
    center_report = facts(measured(args.after, center['model']))
    original_center_pairs = {tuple(sorted((i, 25))) for i in (9, 10, 12, 15, 17, 18, 22, 27)}
    if (center.get('confirmed_true', 0) < 8
            or not original_center_pairs <= set(center_report['confirmed'])):
        raise RuntimeError(f'Center must retain its eight true pairs: {center}')
    center['original_eight_retained'] = True
    repros = []
    for source in read(tracked / 'repros-manifest.json'):
        name = source['file']
        a = facts(measured(args.before, name))
        b = facts(measured(args.after, name))
        repros.append(dict(model=name, before=a, after=b))
        if name.startswith(('01-', '04-')):
            if b['outcome'] != 'pass' or b['confirmed']:
                raise RuntimeError(f'Clearance repro regressed: {name}')
            model = args.corpus / 'repros' / name
            scene = args.output / (name + '.scene.json')
            with scene.open('w') as out, scene.with_suffix('.stderr').open('w') as err:
                subprocess.run([str(args.scene_binary.resolve()), str(model)], stdout=out, stderr=err, check=True)
            reference = (tracked / 'reference/04-mesh-contact-pair.occt.json'
                         if name.startswith('04-') else two_part_reference(model, args.output))
            exact = compare(model, scene, args.after / (name + '.burr.json'), reference, True)
            if exact['extra_pairs'] or exact['missing_pairs']:
                raise RuntimeError('Contact repro pair comparison regressed')
            (args.output / (name + '.comparison.json')).write_text(json.dumps(exact, indent=2) + '\n')
            repros[-1]['comparison_mode'] = exact['comparison_mode']
        else:
            if b['outcome'] != 'incomplete' or b['confirmed']:
                raise RuntimeError(f'Face-only repro must remain incomplete: {name}')
            if not any(r['code'] == 'assembly_required' for r in b['reasons']):
                raise RuntimeError(f'Face-only repro needs an assembly refusal: {name}')
    summary = dict(models=rows, repros=repros,
                   protected_true=sum(checked[n]['confirmed_true'] for n in protected if 'MiniSB_Bowden' not in n),
                   protected_false=sum(checked[n]['confirmed_false'] for n in protected),
                   measured_binary_sha256=read(next(args.after.glob('*.burr.metrics.json')))['binary_sha256'],
                   baseline_binary_sha256=read(next(args.before.glob('*.burr.metrics.json')))['binary_sha256'],
                   scene_binary_sha256=hashlib.sha256(args.scene_binary.read_bytes()).hexdigest())
    if args.current_main:
        summary['current_main_binary_sha256'] = read(next(args.current_main.glob('*.burr.metrics.json')))['binary_sha256']
    if summary['protected_true'] < 64 or summary['protected_false']:
        raise RuntimeError('Protected pair gate failed')
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')


if __name__ == '__main__':
    main()
