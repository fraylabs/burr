"""Join Rust conversion losses to the STEP entities that defined each face."""
import json, pathlib, re, collections, sys

root = pathlib.Path(__file__).resolve().parent.parent
out = pathlib.Path(sys.argv[1]) if len(sys.argv)>1 else root/'artifacts/face-loss/before'
records = [json.loads(line) for line in (out/'census.jsonl').read_text().splitlines()] if (out/'census.jsonl').exists() else []
for stderr in out.glob('*.stderr'):
    model = root/'artifacts/corpus2/models'/stderr.name.removesuffix('.stderr')
    if not model.is_file(): continue
    for line in stderr.read_text().splitlines():
        try: diagnostic = json.loads(line)
        except ValueError: continue
        if not isinstance(diagnostic, dict) or 'terminal_reason' not in diagnostic: continue
        records.append({'model':str(model),'face':diagnostic.get('source_face_id'),
                        'reason':diagnostic['terminal_reason'], 'stage':'tessellation',
                        'diagnostic':diagnostic})
results = []
for row in records:
    row['model'] = pathlib.Path(row['model']).name
for model, rows in __import__('itertools').groupby(sorted(records,key=lambda r:r['model']),key=lambda r:r['model']):
    rows = list(rows)
    source = pathlib.Path(model)
    if not source.is_file(): source = root/'artifacts/corpus2/models'/source.name
    text = source.read_bytes().decode('latin1')
    entities = {int(id): body.strip() for id,body in re.findall(r'^\s*#(\d+)\s*=\s*(.*?);',text,re.M|re.S)}
    def refs(id): return [int(v) for v in re.findall(r'#(\d+)', entities.get(id,''))]
    def kind(id):
        body=entities.get(id,'')
        return re.findall(r'([A-Z][A-Z_0-9]*)\s*\(',body)
    edges = {r['edge']:r['edge_error'] for r in rows if 'edge' in r}
    surfaces = {r['face']:r['surface_error'] for r in rows if 'surface_error' in r}
    for row in rows:
        if 'reason' not in row: continue
        face = row['face']
        chain = refs(face)
        surface = chain[-1] if chain else None
        edge_ids = []
        for bound in chain[:-1]:
            for loop in refs(bound):
                for use in refs(loop):
                    edge_ids.extend(refs(use) if 'ORIENTED_EDGE' in kind(use) else [use])
        edge_ids = sorted(set(edge_ids))
        row.update(model=source.name,surface=surface,surface_types=kind(surface),surface_error=surfaces.get(face),edges=[{'id':id,'types':kind(refs(id)[-1]) if refs(id) else [],'strict_conversion_error':edges.get(id)} for id in edge_ids])
        results.append(row)
(out/'causes.json').write_text(json.dumps(results,indent=2)+'\n')
counts=collections.Counter((pathlib.Path(r['model']).name,r['reason'],','.join(r['surface_types'])) for r in results)
for key,count in sorted(counts.items()): print(count,*key,sep='\t')
