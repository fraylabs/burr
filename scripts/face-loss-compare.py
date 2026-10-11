"""Compare retained finding/contact/unresolved pair identities and cold timings."""
import argparse
import json
import pathlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('before', type=pathlib.Path)
parser.add_argument('after', type=pathlib.Path)
args = parser.parse_args()


def reports(directory):
    result = {}
    for path in directory.glob('*.json'):
        data = json.loads(path.read_text())
        if isinstance(data, dict) and 'report' in data:
            result[path.name] = data
    return result


def pairs(report, field):
    return {(pair['id'], pair['code']) for pair in report.get(field, [])}


before, after = reports(args.before), reports(args.after)
rows = []
for name in sorted(before.keys() & after.keys()):
    left, right = before[name], after[name]
    row = {
        'model': name.removesuffix('.json'),
        'outcomes': [left['report']['outcome'], right['report']['outcome']],
        'seconds': [left['seconds'], right['seconds']],
        'pair_changes': {},
    }
    for field in ('findings', 'contact_pairs', 'unresolved_pairs'):
        old, new = pairs(left['report'], field), pairs(right['report'], field)
        row['pair_changes'][field] = {
            'added': sorted(new - old),
            'removed': sorted(old - new),
        }
    rows.append(row)
print(json.dumps({
    'compared': len(rows),
    'only_before': sorted(before.keys() - after.keys()),
    'only_after': sorted(after.keys() - before.keys()),
    'models': rows,
}, indent=2))
