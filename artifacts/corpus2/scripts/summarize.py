import pathlib,json,csv,collections,argparse
parser=argparse.ArgumentParser(description="Summarize terminal measurements without treating partial pair scans as complete.")
parser.add_argument("--root",type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1])
args=parser.parse_args()
root=args.root.resolve()
rows=[]
for source in csv.DictReader((root/'sources.csv').open()):
 name=pathlib.Path(source['file']).name;prefix=root/'logs'/name
 def read(suffix):
  p=pathlib.Path(str(prefix)+suffix)
  return json.loads(p.read_text()) if p.exists() else {}
 metric=read('.burr.metrics.json');om=read('.occt.metrics.json');b=read('.burr.json').get('report',{});o=read('.occt.json');comparison=read('.comparison.json');meta=om.get('load_metadata',o)
 proof_path=root/'rerun-0400/logs'/(name+'.removed-proof.json')
 proof=json.loads(proof_path.read_text()) if proof_path.exists() else {}
 independent_false=[p for p in proof.get('checked',[]) if all(p.get('source_valid',[])) and len(p.get('source_valid',[]))==2 and len(p.get('mapping_old',[]))==2 and p.get('common_volume_mm3',float('inf'))<=p.get('threshold_mm3',0)]
 if not metric or not om:status='pending'
 elif metric.get('timeout') or metric.get('memory_cap_exceeded'):status='timeout'
 elif metric.get('error') or not b.get('outcome'):status='crash'
 elif comparison.get('extra_pairs') or independent_false:status='false positive'
 elif comparison.get('missing_pairs') and b.get('pair_set_complete') and comparison.get('occurrence_mapping_complete'):status='false negative'
 elif b.get('outcome')=='incomplete' or not b.get('pair_set_complete') or comparison.get('refused') or not comparison or comparison.get('reference_scope')!='all_pairs' or not comparison.get('occurrence_mapping_complete'):status='incomplete'
 else:status='correct'
 rows.append(dict(source=source,name=name,metric=metric,occt_metric=om,burr=b,occt=o,comparison=comparison,parts=meta.get('parts'),status=status,independent_false_pairs=independent_false,independent_proof=str(proof_path) if independent_false else None))
(root/'analysis.json').write_text(json.dumps(rows,indent=2))
print('CLASSES',dict(collections.Counter(r['status'] for r in rows)))
print('CENSUS',[(r['name'],r['parts']) for r in sorted(rows,key=lambda r:r['parts'] or 0,reverse=True)][:7])
print('PRODUCT REASONS',dict(collections.Counter(reason['code'] for r in rows for reason in {reason['code']:reason for reason in r['burr'].get('incomplete_reasons',[])}.values())))
for r in rows:
 c=r['comparison']
 if c.get('extra_pairs'):print('FALSE',r['name'],[f['pair'] for f in c['extra_pairs']])
 if c.get('refused'):print('REFUSED',r['name'],c['refused'])
