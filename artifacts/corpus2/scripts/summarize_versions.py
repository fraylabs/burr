"""Compare released versions without overwriting baseline or certifying unknown pairs."""
import argparse
import collections
import csv
import json
import pathlib


def read(path):
    return json.loads(path.read_text()) if path.exists() else {}


def summarize(root, evidence, limit):
    rows = []
    for source in list(csv.DictReader((root / 'sources.csv').open()))[:limit]:
        name = pathlib.Path(source['file']).name
        old = read(root / 'logs' / (name + '.burr.json')).get('report', {})
        new = read(evidence / (name + '.burr.json')).get('report', {})
        comparison = read(evidence / (name + '.comparison.json'))
        contacts = read(evidence / (name + '.contacts.json'))
        old_codes = sorted({r['code'] for r in old.get('incomplete_reasons', [])})
        new_codes = sorted({r['code'] for r in new.get('incomplete_reasons', [])})
        rows.append(dict(model=name, baseline_outcome=old.get('outcome'),
                         baseline_coordinate_limit='below_coordinate_resolution' in old_codes,
                         outcome=new.get('outcome'), pair_set_complete=new.get('pair_set_complete'),
                         checked_pairs=new.get('checked_pair_count'),
                         findings=len(new.get('findings', [])),
                         contacts=len(new.get('contact_pairs', [])),
                         unresolved=len(new.get('unresolved_pairs', [])), reasons=new_codes,
                         unresolved_reasons=dict(collections.Counter(p['code'] for p in new.get('unresolved_pairs', []))),
                         comparison=comparison, contact_verification=contacts))
    coordinates = [r for r in rows if r['baseline_coordinate_limit']]
    summary = dict(selected=len(rows), measured=sum(r['outcome'] is not None for r in rows),
                   baseline_coordinate_models=len(coordinates),
                   coordinate_models_measured=sum(r['outcome'] is not None for r in coordinates),
                   coordinate_models_now_pair_complete=sum(r['pair_set_complete'] is True for r in coordinates),
                   coordinate_reason_removed=sum(r['outcome'] is not None and 'below_coordinate_resolution' not in r['reasons'] for r in coordinates),
                   coordinate_models_stopped_at_import=sum(r['checked_pairs'] == 0 and 'step_faces_lost' in r['reasons'] for r in coordinates),
                   contacts_reported=sum(r['contacts'] for r in rows),
                   contacts_checked_exact_zero=sum(x.get('exactly_zero') is True for r in rows for x in r['contact_verification'].get('checked', [])),
                   nonzero_contact_pairs=sum(len(r['contact_verification'].get('nonzero_pairs', [])) for r in rows))
    (evidence.parent / 'version-summary.json').write_text(json.dumps(dict(summary=summary, models=rows), indent=2) + '\n')
    lines = ['| Model | 0.39.0 coordinate limit | 0.40.0 verdict | Pair set complete | Interferences matched / extra / missing | Contact Common zeros / reported (source proof separate) | Unresolved pairs and reasons |',
             '|---|---|---|---|---|---|---|']
    for r in rows:
        c = r['comparison']; v = r['contact_verification']
        pairs = 'pending' if not c else ('refused' if c.get('refused') else f"{len(c['matched_pairs'])} / {len(c['extra_pairs'])} / {len(c['missing_pairs'])}")
        if c and not c.get('refused') and (not c.get('occurrence_mapping_complete') or c.get('reference_scope') != 'all_pairs'):
            pairs += ' (subset)'
        zero = sum(x.get('exactly_zero') is True for x in v.get('checked', []))
        verification = f"{zero} / {r['contacts']}"
        if not v or not v.get('complete'):
            verification += ' (unverified remainder)'
        if v.get('nonzero_pairs'):
            verification += ' **NONZERO**'
        reasons = ', '.join(f'{k}: {n}' for k, n in r['unresolved_reasons'].items())
        if not reasons:
            reasons = ', '.join(r['reasons']) or 'none'
        lines.append(f"| {r['model']} | {'yes' if r['baseline_coordinate_limit'] else 'no'} | {r['outcome'] or 'pending'} | {r['pair_set_complete']} | {pairs} | {verification} | {r['unresolved']} ({reasons}) |")
    (evidence.parent / 'version-table.md').write_text('\n'.join(lines) + '\n')
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, required=True)
    parser.add_argument('--evidence', type=pathlib.Path, required=True)
    parser.add_argument('--limit', type=int, default=25)
    args = parser.parse_args()
    summarize(args.root, args.evidence, args.limit)
