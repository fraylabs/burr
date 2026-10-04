"""Compare integrated release measurements with the recorded 0.35/OCCT baseline."""
import argparse, collections, csv, datetime, json, pathlib, re

parser = argparse.ArgumentParser()
parser.add_argument('--baseline', type=pathlib.Path, required=True)
parser.add_argument('--corpus', type=pathlib.Path, required=True)
parser.add_argument('--logs', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--burr-head', required=True)
parser.add_argument('--look-head', required=True)
args = parser.parse_args()

def read(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return {}

def number(value):
    if value is None:
        return '—'
    return f'{value:.6f}' if value < .001 else f'{value:.3f}'

def classification(verdict, reference):
    if verdict not in ('pass', 'fail') or reference not in ('pass', 'fail'):
        return 'incomplete'
    return 'correct' if verdict == reference else 'false positive' if verdict == 'fail' else 'false negative'

def measured(name):
    result = read(args.logs / (name + '.burr.json'))
    metrics = read(args.logs / (name + '.burr.metrics.json'))
    if not metrics:
        raise RuntimeError(f'Missing measured run: {name}')
    report = result.get('report', {})
    loaded = metrics.get('load_metadata', {})
    stats = result.get('step_import', loaded.get('step_import'))
    if stats is None and result.get('error'):
        lost_match = re.search(r'(\d+) of (\d+) STEP faces were lost', result['error'])
        if lost_match:
            stats = dict(lost_faces=int(lost_match[1]), declared_faces=int(lost_match[2]))
    verdict = report.get('outcome', 'timeout' if metrics.get('timeout') else 'load error' if result.get('error') else 'crash')
    diagnostics = collections.Counter()
    diag_path = args.logs / (name + '.diagnostics.jsonl')
    if diag_path.exists():
        for line in diag_path.read_text().splitlines():
            entry = json.loads(line)
            diagnostics[entry.get('refusal_tag', entry.get('terminal_reason', 'tessellation'))] += 1
    return dict(name=name, burr_verdict=verdict, burr_parts=result.get('parts', loaded.get('parts')),
                burr_pairs=len(report.get('findings', [])),
                load_s=result.get('load_s', loaded.get('load_s')), check_s=result.get('check_s'),
                peak_rss_mib=metrics['peak_rss_mib'], step_import=stats,
                incomplete_reasons=report.get('incomplete_reasons', []),
                structure_errors=result.get('structure_errors', loaded.get('structure_errors', [])),
                error=result.get('error'), diagnostic_tags=dict(diagnostics), metrics=metrics)

rows = []
for before in read(args.baseline):
    after = measured(before['name'])
    after.update(file=before['file'], size_mib=before['size_mib'], before=before,
                 occt_parts=before['occt_parts'], occt_verdict=before['occt_verdict'],
                 occt_pairs=before['occt_pairs'], occt_pair_scan_complete=before['occt_pair_scan_complete'])
    after['classification'] = classification(after['burr_verdict'], before['occt_verdict'])
    rows.append(after)
counts = collections.Counter(row['classification'] for row in rows)
verdicts = collections.Counter(row['burr_verdict'] for row in rows)

lines = ['# Burr 0.36 real-assembly integration', '',
    f'Measured all {len(rows)} original multipart STEP models and the six original repros; report updated {datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")}.', '',
    f'{counts["correct"]} of {len(rows)} models now agree with the conclusive OCCT assembly verdict, compared with 4 before. Classifications: {dict(counts)}. Burr verdicts: {dict(verdicts)}.', '',
    'Classification describes the overall assembly verdict, not pair-list equivalence. A Burr or OCCT incomplete/timeout result remains **incomplete**; agreement between two incomplete results does not establish correctness.', '',
    f'Integrated Burr code: `{args.burr_head}` (based on merged #37); Look fork branch `burr`: `{args.look_head}`. Baseline Burr: `29dff12da0669a2e82c7283bf8a4695c02069825` (0.35.0), Look: `dc6687c5a579cee5cdfe91411d30bb71d19a5ac6`.', '',
    '## Results', '',
    '| Assembly | MiB | Parts before → after / OCCT | Faces lost before → after | Load s before → after | Check s before → after | Peak MiB after | Burr before → after (pairs after) | OCCT (pairs) | Classification before → after |',
    '|---|---:|---:|---:|---:|---:|---:|---|---|---|']
for row in rows:
    before = row['before']
    name = row['name'].replace('__', ' / ').replace('_', ' ')
    old_parts = '—' if before['burr_parts'] is None else str(before['burr_parts'])
    new_parts = '—' if row['burr_parts'] is None else str(row['burr_parts'])
    old_loss = '≥' + str(before['partial_face_refusals']) if before.get('partial_face_refusals') else str(before['dropped_faces']) if before['dropped_faces'] is not None else '—'
    new_loss = str(row['step_import']['lost_faces']) if row['step_import'] else '—'
    load_before = number(before['load_s']) if before['load_s'] is not None else f'crash {number(before["elapsed_s"])}'
    check_before = number(before['check_s']) if before['check_s'] is not None else f'~{number(before["elapsed_s"] - (before["load_s"] or 0))} capped' if before['burr_verdict'] == 'timeout' else 'not reached'
    pairs = str(row['occt_pairs']) if row['occt_pairs'] is not None else '—'
    if not row['occt_pair_scan_complete'] and row['occt_pairs']:
        pairs = '≥' + pairs + '; partial scan'
    lines.append(f'| {name} | {row["size_mib"]:.2f} | {old_parts} → {new_parts} / {row["occt_parts"]} | {old_loss} → {new_loss} | {load_before} → {number(row["load_s"])} | {check_before} → {number(row["check_s"])} | {number(row["peak_rss_mib"])} | {before["burr_verdict"]} → {row["burr_verdict"]} ({row["burr_pairs"]}) | {row["occt_verdict"]} ({pairs}) | {before["classification"]} → {row["classification"]} |')
lines += ['', '## What improved and what remains', '',
    'All 13 previously flattened inputs now match the OCCT component count; 20 of 21 counts match overall. The full robot still falls back to one mesh and explicitly reports the unresolved source graph. There are no crashes or timeouts in the integrated 21-model run. The nullable-dollar repro now retains both occurrences, and both reduced Apollo faces render again. The two-part contact repro changes from false interference to a correct pass.', '',
    'Source-face loss falls from 16,251 to 96 on the inputs whose baseline import completed. Across all 21 integrated models there are 134 losses in nine models; the four previously crashing inputs did not have complete baseline loss counts, so 134 is not directly comparable with 16,251.', '',
    'The remaining import diagnoses are concrete: all 26 surface-conversion refusals are source TOROIDAL_SURFACE records with negative major radii; 42 Switchwire faces fail wire closure; the other 66 losses are periodic-chart parity/constraint/degeneracy refusals (33 ContradictoryDualParity, 12 ConstraintInsertionIncomplete, 11 NoOddParityRegion, 9 RejectedDegenerate, 1 ConstraintRoleMissing). Negative-radius handling is now a refusal, not support for that surface convention. V0 retains its 1,691 occurrences but includes two definitions with no source geometry.', '',
    'The full robot graph fails while following two source SHAPE_REPRESENTATION_RELATIONSHIP records to MANIFOLD_SURFACE_SHAPE_REPRESENTATION (#55392 and #288197). Those entity variants are absent from Truck’s shape-representation holder table; a read-only probe confirms 197 decoded shape representations (107 basic + 90 advanced BREP), omitting the two manifold-surface representations. Supporting that representation variant is the next structure fix; this integration reports the loss explicitly instead of declaring a single flattened body safe.', '',
    'Several meshes remain open even though every declared face contributes triangles: Bowden, Fly, both Apollo enclosures, and the arcade button. A read-only probe of production mesh preparation finds 120 boundary edges and 100 nonmanifold edges in Apollo flat Solid #15; corner Solid #15 has 128 boundary edges and 100 nonmanifold edges. Their other solids are closed. Fly has nonmanifold edges in five of its six definitions, including a bracket with zero boundary edges but two nonmanifold edges. The best working guess is nonconforming boundary sampling or overlapping triangulation; simple near-vertex welding is not an adequate explanation for Apollo, whose nearest distinct boundary vertices are over 0.04 mm apart. See topology-0.36.json and the reproducible corpus-topology.rs probe. Zero lost faces does not prove a closed mesh. The arcade spring is also invalid in the OCCT reference. Bowden has no conclusive exact verdict because its recorded OCCT run timed out.', '',
    'A correct overall fail still does not certify its pair list. The cover reports 17 pairs versus OCCT 22, and the Adafruit switch reports 4 versus 11; those lists need a placement-matched exact comparison. Faze4 and the two build123d examples retain the reference pair counts (9, 3, 12). No assembly-level false positive or false negative is observed among the seven conclusive results.', '',
    '## Remaining failures', '']
for row in rows:
    if row['classification'] != 'correct':
        reasons = '; '.join(f'{r["code"]}: {r["message"]}' for r in row['incomplete_reasons']) or row.get('error') or f'Burr {row["burr_verdict"]}; OCCT {row["occt_verdict"]}'
        lines.append(f'- **{row["name"]}** — {reasons}')
lines += ['', '## Repros', '',
    '| Input | Parts before → after | Faces lost after | Load s before → after | Check s before → after | Burr before → after | Interpretation |',
    '|---|---:|---:|---:|---:|---|---|']
repros = []
for file in sorted((args.corpus / 'repros').glob('*.step')):
    before = read(args.output.parent / 'baseline-repros.json').get(file.name, read(pathlib.Path(str(file) + '.json')))
    after = measured(file.name)
    repros.append(dict(file=file.name, before=before, after=after))
    old = before.get('report', {}).get('outcome', 'load error' if before.get('error') else 'crash')
    lost = str(after['step_import']['lost_faces']) if after['step_import'] else '—'
    interpretation = '; '.join(r['code'] for r in after['incomplete_reasons']) or after.get('error') or 'complete check'
    if file.name.startswith('04-'):
        interpretation = 'OCCT pass (0 pairs): ' + classification(after['burr_verdict'], 'pass')
    elif file.name.startswith(('02-', '03-')):
        interpretation = 'Isolated source face, not a two-component clearance test. ' + interpretation
    lines.append(f'| {file.name} | {before.get("parts", "—")} → {after["burr_parts"] if after["burr_parts"] is not None else "—"} | {lost} | {number(before.get("load_s"))} → {number(after["load_s"])} | {number(before.get("check_s"))} → {number(after["check_s"])} | {old} → {after["burr_verdict"]} | {interpretation} |')
lines += ['', '## Measurement limits', '',
    '- Fresh process per input, one model at a time, release profile, four Cargo jobs, shared `/tmp/burr-build.lock` and `/tmp/burr-target`. Source SHA-256 values are verified before measurement. Load covers read/parse/tessellation/scene compilation; check covers production `analyze_scene` and excludes the browser, API scene clone and server cache. OS filesystem caches are uncontrolled.',
    '- The integrated harness records source-definition face statistics directly from Look, counting shared source shells once. The before figures use the baseline diagnostic warning counts; baseline crashes only give partial refusal counts. The isolated negative-torus face is expected to return a typed no-geometry load error rather than panic.',
    '- 600 seconds and 9 GiB per process; caps and load errors remain incomplete. Peak RSS is OS `wait4` maximum for load plus check. This is the same shared Apple M1 Pro, 16 GiB, macOS 27.0 machine; timings are observations under uncontrolled system load, not isolated performance claims.',
    '- Reused exact reference: CadQuery 2.8.0 / OCP 7.9.3.1.1, XCAF leaf occurrences with world placements, BREP Common for strict AABB candidates. Flat one-leaf multipart exports are split by solid. Positive common volume must exceed max(1e-6 mm³, smaller-part volume × 1e-9); touching is not interference. Large reference scans stop at one valid positive-volume witness, so their pair count is a lower bound. MiniSB Bowden reference timed out; the arcade-button spring is OCCT-invalid. Neither can be certified correct against that reference.',
    '- Corpus models and raw repro geometry remain external measurement inputs under their original licences. [sources.csv](sources.csv) records URLs, licences, repository revisions and SHA-256 values. Burr receives no GPL/CC CAD fixture in this change. The nullable Look regression is synthetic; the two source-derived face snippets in Look have separate source notices/licences.',
    '', '## Reproduce', '', '```sh',
    '# Put the original corpus at CORPUS, using sources.csv and the original reducers.',
    'mkdir -p examples',
    'cp artifacts/corpus/scripts/corpus-bench.rs examples/corpus-bench.rs',
    'until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done',
    'CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target cargo build --release --locked --bin burr --example corpus-bench',
    'rmdir /tmp/burr-build.lock',
    'until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done',
    'CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target /tmp/burr-occt-venv/bin/python artifacts/corpus/scripts/run_integrated.py --corpus "$CORPUS" --manifest artifacts/corpus/baseline-0.35.json --output artifacts/corpus/logs-0.36 --binary /tmp/burr-target/release/examples/corpus-bench',
    'rmdir /tmp/burr-build.lock', '```', '',
    'The runner uses the recorded baseline JSON as its 21-model manifest and appends all six `.step` repros. It skips completed runs; choose an empty output directory for a new measurement. Raw integrated reports, metrics and diagnostic JSONL files are retained under `logs-0.36/`. Aggregate before/after rows are in [results-0.36.json](results-0.36.json).', '']
args.output.write_text('\n'.join(lines))
args.output.with_suffix('.json').write_text(json.dumps(dict(burr_head=args.burr_head, look_head=args.look_head, counts=dict(counts), verdicts=dict(verdicts), models=rows, repros=repros), indent=2) + '\n')
print(dict(counts), dict(verdicts))
