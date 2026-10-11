import sys,json,time,pathlib
start=time.monotonic()
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepCheck import BRepCheck_Analyzer
import cadquery as cq
path=sys.argv[1]
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[2]/"corpus/scripts"))
from occt_components import components
leaf=components(path)
leaf_count=len(leaf)
solids=[cq.Shape.cast(s) for n,s in leaf]
flat_multibody=False
if len(solids)==1 and len(solids[0].Solids())>1:
 flat_multibody=True
 solids=solids[0].Solids()
 leaf=[('solid:'+str(i),s.wrapped) for i,s in enumerate(solids)]
valid=[bool(BRepCheck_Analyzer(s.wrapped).IsValid()) for s in solids]
boxes=[];bounding_errors=[]
for i,s in enumerate(solids):
 try:boxes.append(s.BoundingBox())
 except Exception as e:
  boxes.append(None);valid[i]=False;bounding_errors.append([i,str(e)])
meta=dict(solids=sum(len(s.Solids()) for s in solids),parts=len(solids),leaf_components=leaf_count,flat_multibody=flat_multibody,names=[n for n,s in leaf],faces=sum(len(s.Faces()) for s in solids),invalid_solids=sum(not x for x in valid),bounding_errors=bounding_errors,import_s=time.monotonic()-start)
print('OCCT_LOADED '+json.dumps(meta),file=sys.stderr,flush=True)
# Positive common volume; face/edge contact is not interference.
# Absolute tolerance mm^3 plus relative numerical floor.
findings=[];errors=list(bounding_errors);candidate=0
witness_only=pathlib.Path(path).stat().st_size>10_000_000 or len(solids)>150
pair_check_complete=True
for i,a in enumerate(solids):
 for j in range(i+1,len(solids)):
  x,y=boxes[i],boxes[j]
  if x is None or y is None:continue
  if any(min(getattr(x,k+'max'),getattr(y,k+'max'))-max(getattr(x,k+'min'),getattr(y,k+'min'))<=1e-7 for k in 'xyz'):continue
  candidate+=1
  try:
   common=BRepAlgoAPI_Common(a.wrapped,solids[j].wrapped);common.Build()
   if not common.IsDone():errors.append([i,j,'not_done']);continue
   v=abs(cq.Shape.cast(common.Shape()).Volume())
   threshold=max(1e-6,min(abs(a.Volume()),abs(solids[j].Volume()))*1e-9)
   if v>threshold:
    findings.append(dict(pair=[i,j],volume_mm3=v,threshold_mm3=threshold));print('OCCT_OVERLAP '+json.dumps(findings[-1]),file=sys.stderr,flush=True)
    if witness_only and valid[i] and valid[j]:pair_check_complete=False;break
  except Exception as e:errors.append([i,j,str(e)])
  if candidate%100==0:print('OCCT_PROGRESS '+str(candidate),file=sys.stderr,flush=True)
 if not pair_check_complete:break
print(json.dumps(dict(**meta,pair_check_complete=pair_check_complete,elapsed_s=time.monotonic()-start,candidates=candidate,findings=findings,errors=errors,outcome='fail' if not pair_check_complete else ('incomplete' if errors or not all(valid) else ('fail' if findings else 'pass')))))
