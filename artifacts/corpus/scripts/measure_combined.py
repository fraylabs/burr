"""Gate combined import changes against a freshly measured Burr baseline.

Run under the shared build lock in the documented OCCT environment. Bench
outputs come from run_integrated.py; detailed output stays local. No matching
or Boolean tolerances are changed. Pair-limited references certify reported
pairs only, and cannot certify the rest of an assembly.
"""
import argparse
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
                                                       message=f"{metrics['cap']} cap; no completed Burr check")])
        return result
    return read(directory / (name + '.burr.json'))


def pair(finding):
    return tuple(sorted(c['occurrence_index'] for c in finding['components']))


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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ('corpus', 'reference-logs', 'before', 'after', 'scene-binary', 'output'):
        parser.add_argument('--' + option, type=pathlib.Path, required=True)
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
        changed = a != b
        outcome = b['outcome']
        if outcome in ('pass', 'fail') and source['occt_verdict'] in ('pass', 'fail') and outcome != source['occt_verdict']:
            raise RuntimeError(f'Wrong conclusive verdict: {name}')
        if outcome == 'pass' and source['occt_verdict'] != 'pass':
            raise RuntimeError(f'Pass has no completed exact clearance reference: {name}')
        removed = set(a['confirmed']) - set(b['confirmed'])
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
                   unresolved=len(b['unresolved']), incomplete_reasons=b['reasons'],
                   structure_errors=b['structure_errors'])
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
                           occt_positives_unconfirmed=len(exact['missing_pairs']))
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
    if center.get('confirmed_true') != 8:
        raise RuntimeError(f'Center must retain its eight true pairs: {center}')
    repros = []
    for source in read(tracked / 'repros-manifest.json'):
        name = source['file']
        a = facts(measured(args.before, name))
        b = facts(measured(args.after, name))
        repros.append(dict(model=name, before=a, after=b))
        if name.startswith('04-') and (b['outcome'] != 'pass' or b['confirmed']):
            raise RuntimeError('Contact repro regressed')
        if name.startswith('04-'):
            model = args.corpus / 'repros' / name
            scene = args.output / (name + '.scene.json')
            with scene.open('w') as out, scene.with_suffix('.stderr').open('w') as err:
                subprocess.run([str(args.scene_binary.resolve()), str(model)], stdout=out, stderr=err, check=True)
            exact = compare(model, scene, args.after / (name + '.burr.json'),
                            tracked / 'reference/04-mesh-contact-pair.occt.json', True)
            if exact['extra_pairs'] or exact['missing_pairs']:
                raise RuntimeError('Contact repro pair comparison regressed')
            (args.output / (name + '.comparison.json')).write_text(json.dumps(exact, indent=2) + '\n')
    summary = dict(models=rows, repros=repros,
                   protected_true=sum(checked[n]['confirmed_true'] for n in protected if 'MiniSB_Bowden' not in n),
                   protected_false=sum(checked[n]['confirmed_false'] for n in protected),
                   measured_binary_sha256=read(next(args.after.glob('*.burr.metrics.json')))['binary_sha256'],
                   baseline_binary_sha256=read(next(args.before.glob('*.burr.metrics.json')))['binary_sha256'],
                   scene_binary_sha256=hashlib.sha256(args.scene_binary.read_bytes()).hexdigest())
    if summary['protected_true'] < 64 or summary['protected_false']:
        raise RuntimeError('Protected pair gate failed')
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')


if __name__ == '__main__':
    main()
