"""Trace removed interference findings into newer lists, then exact-check them.

List transitions are preliminary until both released mesh occurrences match
the same source occurrences under the existing strict identity/surface checks.
No safe/absent transition is hidden by an incomplete overall pair set.
"""
import argparse
import json
import pathlib
import sys


def pair(entry):
    return tuple(sorted(c['occurrence_index'] for c in entry['components']))


def audit(model, baseline, newer, scripts):
    name = model.name
    old = json.loads((baseline / (name + '.burr.json')).read_text())['report']
    new = json.loads((newer / (name + '.burr.json')).read_text())['report']
    findings = {pair(f): f for f in new['findings']}
    unresolved = {pair(f): f for f in new.get('unresolved_pairs', [])}
    contacts = {pair(f): f for f in new.get('contact_pairs', [])}
    removed = [f for f in old['findings'] if pair(f) not in findings]
    destination = newer / (name + '.removed-proof.json')
    result = dict(model=name, old_outcome=old['outcome'], new_outcome=new['outcome'],
                  new_pair_set_complete=new['pair_set_complete'],
                  removed_count=len(removed), complete=False, checked=[], refused=[])

    def save():
        destination.write_text(json.dumps(result, indent=2) + '\n')

    save()
    sys.path.insert(0, str(scripts))
    import cadquery as cq
    from compare_pairs import compare
    from occt_components import components
    from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
    from OCP.BRepCheck import BRepCheck_Analyzer
    exact = components(model)
    if len(exact) == 1 and len(cq.Shape.cast(exact[0][1]).Solids()) > 1:
        exact = [('solid:' + str(i), s.wrapped)
                 for i, s in enumerate(cq.Shape.cast(exact[0][1]).Solids())]
    shapes = [cq.Shape.cast(s) for _, s in exact]
    ref = newer / (name + '.removed-mapping-reference.json')
    ref.write_text(json.dumps(dict(names=[n for n, _ in exact], findings=[], pair_check_complete=True)))
    selected = newer / (name + '.removed-mapping-report.json')

    def mapping(entries, folder):
        selected.write_text(json.dumps(dict(report=dict(findings=entries))))
        mapped = compare(model, folder / (name + '.scene.json'), selected, ref,
                         reported_pairs_only=True)
        return {r['burr']: r['occt'] for r in mapped['mapping']}, mapped['mapping']

    batch = None
    try:
        batch = (mapping(removed, baseline), mapping(removed, newer))
    except Exception as error:
        result['batch_mapping_refusal'] = str(error)
    for finding in removed:
        indices = pair(finding)
        entry = unresolved.get(indices, contacts.get(indices))
        status = 'unresolved' if indices in unresolved else 'contact_or_separated' if indices in contacts else 'absent'
        record = dict(burr_pair=indices, status=status,
                      reason=entry['code'] if entry else None,
                      new_entry=entry, old_finding=finding)
        try:
            old_map, old_evidence = batch[0] if batch else mapping([finding], baseline)
            new_map, new_evidence = batch[1] if batch else mapping([finding], newer)
            old_pair = sorted(old_map[k] for k in indices)
            new_pair = sorted(new_map[k] for k in indices)
            if old_pair != new_pair:
                raise ValueError('Occurrence pair changed source identity between versions')
            record.update(occt_pair=old_pair,
                          mapping_old=[r for r in old_evidence if r['burr'] in indices],
                          mapping_new=[r for r in new_evidence if r['burr'] in indices])
            a, b = [shapes[k] for k in old_pair]
            valid = [bool(BRepCheck_Analyzer(s.wrapped).IsValid()) for s in (a, b)]
            record['source_valid'] = valid
            if not all(valid):
                raise ValueError('Invalid source; cannot certify exact overlap')
            common = BRepAlgoAPI_Common(a.wrapped, b.wrapped)
            common.Build()
            if not common.IsDone() or not BRepCheck_Analyzer(common.Shape()).IsValid():
                raise ValueError('Common unfinished or invalid')
            volume = abs(cq.Shape.cast(common.Shape()).Volume())
            threshold = max(1e-6, min(abs(a.Volume()), abs(b.Volume())) * 1e-9)
            record.update(common_volume_mm3=volume, threshold_mm3=threshold,
                          occt_positive=volume > threshold,
                          positive_hidden_or_safe=volume > threshold and status != 'unresolved')
            result['checked'].append(record)
        except Exception as error:
            record['refused'] = str(error)
            result['refused'].append(record)
        save()
        print(name, indices, status, record.get('common_volume_mm3', record.get('refused')), flush=True)
    result['complete'] = not result['refused'] and len(result['checked']) == len(removed)
    save()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model', type=pathlib.Path, required=True)
    parser.add_argument('--baseline', type=pathlib.Path, required=True)
    parser.add_argument('--newer', type=pathlib.Path, required=True)
    parser.add_argument('--corpus-scripts', type=pathlib.Path, required=True)
    args = parser.parse_args()
    audit(args.model, args.baseline, args.newer, args.corpus_scripts)
