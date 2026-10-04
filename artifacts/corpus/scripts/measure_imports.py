"""Rerun the corpus and all seven completed pair comparisons after an import fix.

Run this command under /tmp/burr-build.lock, with CARGO_BUILD_JOBS=4 and
CARGO_TARGET_DIR=/tmp/burr-target. Keep --output local; only the compact summary
is intended for version control. Requires the documented OCCT Python environment.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--corpus', type=pathlib.Path, required=True)
parser.add_argument('--reference-logs', type=pathlib.Path, required=True)
parser.add_argument('--binary', type=pathlib.Path, required=True)
parser.add_argument('--scene-binary', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
args = parser.parse_args()
scripts = pathlib.Path(__file__).resolve().parent
tracked = scripts.parent
args.output.mkdir(parents=True, exist_ok=True)
subprocess.run([sys.executable, str(scripts / 'run_integrated.py'),
                '--corpus', str(args.corpus), '--manifest', str(tracked / 'baseline-0.35.json'),
                '--output', str(args.output), '--binary', str(args.binary.resolve())], check=True)
baseline = json.loads((tracked / 'results-0.36.json').read_text())['models']
models = []
for old in baseline:
    name = old['name']
    result = json.loads((args.output / (name + '.burr.json')).read_text())
    outcome = result['report']['outcome']
    if outcome in ('pass', 'fail') and old['occt_verdict'] in ('pass', 'fail') and outcome != old['occt_verdict']:
        raise RuntimeError(f'Wrong conclusive verdict: {name}: {outcome}')
    models.append(dict(model=name, source_sha256=hashlib.sha256((args.corpus / 'models' / name).read_bytes()).hexdigest(), before_parts=old['burr_parts'], after_parts=result['parts'],
                       occt_parts=old['occt_parts'], before_lost_faces=old['step_import']['lost_faces'],
                       after_lost_faces=result['step_import']['lost_faces'],
                       before_outcome=old['burr_verdict'], after_outcome=outcome, occt_outcome=old['occt_verdict'],
                       structure_errors=result['structure_errors']))
pairs = []
for old in json.loads((tracked / 'pair-evidence' / 'summary.json').read_text()):
    name = old['model']
    scene = args.output / (name + '.scene.json')
    with scene.open('w') as out, (args.output / (name + '.scene.stderr')).open('w') as err:
        subprocess.run([str(args.scene_binary.resolve()), str(args.corpus / 'models' / name)],
                       stdout=out, stderr=err, check=True)
    comparison = args.output / (name + '.comparison.json')
    subprocess.run([sys.executable, str(scripts / 'compare_pairs.py'),
                    str(args.corpus / 'models' / name), '--scene', str(scene),
                    '--burr', str(args.output / (name + '.burr.json')),
                    '--occt', str(args.reference_logs / (name + '.occt.json')),
                    '--output', str(comparison), '--reference-complete'], check=True)
    exact = json.loads(comparison.read_text())
    row = dict(model=name, occt_pairs=exact['exact_pair_count'],
               before_true=old['after_true'], before_false=old['after_false'],
               after_true=len(exact['matched_pairs']), after_false=len(exact['extra_pairs']),
               remaining_exact_pairs=len(exact['missing_pairs']))
    pairs.append(row)
    if row['after_false'] or row['after_true'] < row['before_true']:
        raise RuntimeError(f'Pair regression requires investigation: {name}: {row}')
summary = dict(binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),
               scene_binary_sha256=hashlib.sha256(args.scene_binary.read_bytes()).hexdigest(),
               models=models, pairs=pairs)
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
