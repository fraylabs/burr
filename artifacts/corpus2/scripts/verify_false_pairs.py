"""Independently record validity, Common volume and distance for extra pairs.

Invoke under the shared lock after strict compare_release.py. Only uniquely
validated occurrence pairs from comparison evidence are checked.
"""
import pathlib,json,sys,argparse
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--root",type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1])
args=parser.parse_args()
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[2]/'corpus/scripts'))
import cadquery as cq
from occt_components import components
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepCheck import BRepCheck_Analyzer
root=args.root.resolve()
for p in (root/'logs').glob('*.comparison.json'):
 comp=json.loads(p.read_text());extra=comp.get('extra_pairs')
 if not extra:continue
 model=root/'models'/comp['model'];exact=components(model)
 if len(exact)==1 and len(cq.Shape.cast(exact[0][1]).Solids())>1:exact=[('solid:'+str(i),s.wrapped) for i,s in enumerate(cq.Shape.cast(exact[0][1]).Solids())]
 proof=[]
 for finding in extra:
  i,j=finding['pair'];a,b=[cq.Shape.cast(exact[k][1]) for k in (i,j)];common=BRepAlgoAPI_Common(a.wrapped,b.wrapped);common.Build();shape=cq.Shape.cast(common.Shape());proof.append(dict(pair=[i,j],names=[exact[k][0] for k in (i,j)],source_valid=[BRepCheck_Analyzer(s.wrapped).IsValid() for s in (a,b)],common_done=common.IsDone(),common_valid=BRepCheck_Analyzer(shape.wrapped).IsValid(),common_volume_mm3=abs(shape.Volume()),minimum_distance_mm=a.distance(b),threshold_mm3=max(1e-6,min(abs(a.Volume()),abs(b.Volume()))*1e-9)))
 out=p.with_name(model.name+'.false-pair-proof.json');out.write_text(json.dumps(proof,indent=2));print(model.name,proof,flush=True)
