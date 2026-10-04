"""Compare Checks counts and protect every confirmed pair with exact OCCT.

Run after run_integrated.py, under the shared Burr build lock. The ten protected
models have completed exact references. Bowden is explicitly limited to its 18
historical pairs plus every current finding/contact; full completeness is unknown.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess

from compare_pairs import compare


def read(path):
    return json.loads(path.read_text())


def pair(finding):
    return tuple(sorted(c['occurrence_index'] for c in finding['components']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ('corpus', 'before', 'after', 'scene-binary', 'output'):
        parser.add_argument('--' + option, type=pathlib.Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    tracked = pathlib.Path(__file__).resolve().parent.parent
    protected = {r['model']: r for r in read(tracked / 'conforming-mesh/pair-summary.json')}
    rows = []
    confirmed_true = confirmed_false = positive_contacts = 0
    for file in sorted(list((args.corpus / 'models').glob('*')) + list((args.corpus / 'repros').glob('*.step'))):
        if not file.is_file():
            continue
        name = file.name
        before = read(args.before / (name + '.burr.json'))
        after = read(args.after / (name + '.burr.json'))
        old, report = before.get('report', {}), after.get('report', {})
        old_pairs = {pair(f) for f in old.get('findings', [])}
        new_pairs = {pair(f) for f in report.get('findings', [])}
        if old_pairs - new_pairs:
            raise RuntimeError(f'Confirmed pair disappeared: {name}: {old_pairs - new_pairs}')
        row = dict(model=name, source_sha256=hashlib.sha256(file.read_bytes()).hexdigest(),
                   before={k: len(old.get(v, [])) for k, v in [('real', 'findings'), ('contact', 'contact_pairs'), ('unresolved', 'unresolved_pairs')]},
                   after={k: len(report.get(v, [])) for k, v in [('real', 'findings'), ('contact', 'contact_pairs'), ('unresolved', 'unresolved_pairs')]},
                   outcome=report.get('outcome', 'load error'))
        if name in protected:
            import cadquery as cq
            from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
            from OCP.BRepCheck import BRepCheck_Analyzer
            from occt_components import components
            scene = args.output / (name + '.scene.json')
            with scene.open('w') as out:
                subprocess.run([str(args.scene_binary), str(file)], stdout=out, check=True)
            if 'Bowden' in name:
                exact = components(file)
                reference = dict(names=[n for n, _ in exact], findings=[], pair_check_complete=False,
                                 pair_check_scope='reported_pairs')
                reference_path = args.output / 'bowden-occt.json'
                reference_path.write_text(json.dumps(reference))
                mapping_result = compare(file, scene, args.after / (name + '.burr.json'), reference_path,
                                         reported_pairs_only=True)
                mapping = {r['burr']: r['occt'] for r in mapping_result['mapping']}
                previous = read(tracked / 'conforming-mesh/bowden-reported-pairs.json')['pairs']
                checked = {tuple(sorted(p['occt_pair'])) for p in previous}
                checked.update(tuple(sorted(mapping[i] for i in pair(f)))
                               for f in report.get('findings', []) + report.get('contact_pairs', []))
                for p in previous:
                    for i, j in zip(p['burr_pair'], p['occt_pair']):
                        if mapping[i] != j:
                            raise RuntimeError('Bowden historical pair correspondence changed')
                shapes = [cq.Shape.cast(s) for _, s in exact]
                details = []
                for i, j in sorted(checked):
                    if not all(BRepCheck_Analyzer(shapes[k].wrapped).IsValid() for k in (i, j)):
                        raise RuntimeError(f'Invalid Bowden pair: {i}:{j}')
                    common = BRepAlgoAPI_Common(shapes[i].wrapped, shapes[j].wrapped)
                    common.Build()
                    if not common.IsDone() or not BRepCheck_Analyzer(common.Shape()).IsValid():
                        raise RuntimeError(f'Unverified Bowden common: {i}:{j}')
                    volume = abs(cq.Shape.cast(common.Shape()).Volume())
                    floor = max(1e-6, min(abs(shapes[i].Volume()), abs(shapes[j].Volume())) * 1e-9)
                    entry = dict(pair=[i, j], volume_mm3=volume, threshold_mm3=floor, positive=volume > floor)
                    details.append(entry)
                    if entry['positive']:
                        reference['findings'].append(entry)
                reference.update(checked_pairs=sorted(checked), checks=details)
                reference_path.write_text(json.dumps(reference, indent=2) + '\n')
                result = compare(file, scene, args.after / (name + '.burr.json'), reference_path,
                                 reported_pairs_only=True)
                row['reference_scope'] = '18 historical pairs plus all current findings and contacts'
            else:
                result = compare(file, scene, args.after / (name + '.burr.json'),
                                 args.corpus / 'logs' / (name + '.occt.json'), reference_complete=True)
                row['reference_scope'] = 'full exact pair scan'
            row.update(confirmed_true=len(result['matched_pairs']), confirmed_false=len(result['extra_pairs']),
                       positive_contacts=len(result['occt_positive_contacts']))
            if row['confirmed_true'] != protected[name]['confirmed_true'] or row['confirmed_false'] or row['positive_contacts']:
                raise RuntimeError(f'Protected accuracy regression: {row}')
            confirmed_true += row['confirmed_true']
            confirmed_false += row['confirmed_false']
            positive_contacts += row['positive_contacts']
            (args.output / (name + '.comparison.json')).write_text(json.dumps(result, indent=2) + '\n')
            print(name, row, flush=True)
        rows.append(row)
    result = dict(models=rows, zero_false_gate=dict(confirmed_true=confirmed_true, confirmed_false=confirmed_false,
                  occt_positive_contacts=positive_contacts, protected_ten_true=confirmed_true - 9, bowden_true=9,
                  bowden_scope='pair limited; full candidate completeness unknown'))
    if confirmed_true != 73 or confirmed_false or positive_contacts:
        raise RuntimeError(f'Zero-false gate failed: {result["zero_false_gate"]}')
    (args.output / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(result['zero_false_gate'], flush=True)


if __name__ == '__main__':
    main()
