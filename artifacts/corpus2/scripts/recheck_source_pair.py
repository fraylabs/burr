import argparse,json,pathlib,sys,subprocess,time,itertools,math
parser=argparse.ArgumentParser()
parser.add_argument('--model',type=pathlib.Path,required=True)
parser.add_argument('--pair',type=int,nargs=2,required=True)
parser.add_argument('--output',type=pathlib.Path,required=True)
parser.add_argument('--timeout',type=float,default=150)
parser.add_argument('--evidence',type=pathlib.Path,required=True,help='Directory containing logs/model.burr.json and logs/model.scene.json')
parser.add_argument('--corpus-scripts',type=pathlib.Path,required=True)
parser.add_argument('--list',choices=['contact_pairs','findings'],default='contact_pairs')
parser.add_argument('--worker',action='store_true')
args=parser.parse_args()
if not args.worker:
    begin=time.monotonic()
    try:
        code=subprocess.run([sys.executable,str(pathlib.Path(__file__).resolve()),*sys.argv[1:],'--worker'],timeout=args.timeout).returncode
    except subprocess.TimeoutExpired:
        code=124
    metrics=dict(returncode=code,timeout=code==124,elapsed_s=time.monotonic()-begin)
    args.output.with_suffix('.metrics.json').write_text(json.dumps(metrics))
    if code!=0:
        record=json.loads(args.output.read_text()) if args.output.exists() else dict(model=args.model.name,burr_pair=args.pair)
        record.update(status='disputed',phase='finished',error='reference returncode '+str(code))
        args.output.write_text(json.dumps(record,indent=2))
    raise SystemExit(code)

import numpy as np
import cadquery as cq
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.gp import gp_Pnt
logs=args.evidence/'logs'
sys.path.insert(0,str(args.corpus_scripts))
import compare_pairs
from occt_components import components
pair=sorted(args.pair)
report=json.loads((logs/(args.model.name+'.burr.json')).read_text())['report']
contact=next(x for x in report[args.list] if sorted(c['occurrence_index'] for c in x['components'])==pair)
record=dict(model=args.model.name,burr_pair=pair,reported_pair=contact,reported_list=args.list,status='disputed',method='strict source mapping + Common + bounded source classification + independent solid-angle winding',phase='loading')
def save():args.output.write_text(json.dumps(record,indent=2))
save()
source=components(args.model)
mapping_file=args.output.with_suffix('.mapping-report.json')
reference_file=args.output.with_suffix('.mapping-reference.json')
mapping_file.write_text(json.dumps(dict(report=dict(findings=[contact]))))
reference_file.write_text(json.dumps(dict(names=[n for n,_ in source],findings=[],pair_check_complete=True)))
# The unmodified matcher independently imports the same source. No limits relaxed.
try:
    mapping=compare_pairs.compare(args.model,logs/(args.model.name+'.scene.json'),mapping_file,reference_file,reported_pairs_only=True)
except Exception as error:
    record.update(phase='finished',refused=str(error));save();raise SystemExit(0)
index={r['burr']:r['occt'] for r in mapping['mapping']}
occt_pair=sorted(index[i] for i in pair)
shapes=[cq.Shape.cast(source[i][1]) for i in occt_pair]
record.update(occt_pair=occt_pair,mapping=mapping['mapping'],source_names=[source[i][0] for i in occt_pair],source_valid=[s.isValid() for s in shapes],solid_counts=[len(s.Solids()) for s in shapes])
save()
if not all(record['source_valid']) or any(n==0 for n in record['solid_counts']):
    record.update(phase='finished',refused='Invalid or nonsolid source component');save();raise SystemExit(0)
common=BRepAlgoAPI_Common(shapes[0].wrapped,shapes[1].wrapped);common.SetRunParallel(False);common.SetNonDestructive(True);common.Build()
record['common']=dict(done=common.IsDone())
common_shape=None
if common.IsDone():
    common_shape=cq.Shape.cast(common.Shape())
    record['common'].update(valid=common_shape.isValid(),volume_mm3=abs(common_shape.Volume()),solids=len(common_shape.Solids()))
save()
closest=BRepExtrema_DistShapeShape(shapes[0].wrapped,shapes[1].wrapped);closest.Perform()
seeds=[]
if closest.IsDone():
    record['source_distance_mm']=closest.Value()
    for i in range(1,min(closest.NbSolution(),3)+1):
        a,b=closest.PointOnShape1(i),closest.PointOnShape2(i)
        seeds.append([(a.X()+b.X())/2,(a.Y()+b.Y())/2,(a.Z()+b.Z())/2])
boxes=[s.BoundingBox() for s in shapes]
lower=[max(getattr(s,a+'min') for s in boxes) for a in 'xyz']
upper=[min(getattr(s,a+'max') for s in boxes) for a in 'xyz']
seeds.append([(a+b)/2 for a,b in zip(lower,upper)])
if common_shape is not None:
    seeds.extend(list(s.Center().toTuple()) for s in common_shape.Solids()[:2])
seeds=list({tuple(round(v,11) for v in p):p for p in seeds}.values())
# Preserve the released Burr hypothesis as extra seeds after converting its
# viewer coordinates (x, z, -y) back to STEP coordinates (x, y, z).
witness=contact.get('witness',{})
viewer_points=[witness[k] for k in ('point','start','end') if k in witness]
if 'start' in witness and 'end' in witness:
    viewer_points.append([(a+b)/2 for a,b in zip(witness['start'],witness['end'])])
seeds.extend([p[0],-p[2],p[1]] for p in viewer_points)
seeds=list({tuple(round(v,11) for v in p):p for p in seeds}.values())
record['seed_points']=seeds
classifiers=[[BRepClass3d_SolidClassifier(s.wrapped) for s in component.Solids()] for component in shapes]
boundaries=[cq.Compound.makeCompound(s.Faces()) for s in shapes]
points=[]
for seed in seeds:
    for offset in itertools.product([-0.002,0,0.002],repeat=3):
        p=[a+b for a,b in zip(seed,offset)];states=[]
        for component in classifiers:
            ss=[]
            for classifier in component:
                classifier.Perform(gp_Pnt(*p),1e-9);ss.append(str(classifier.State()))
            states.append('IN' if any(s.endswith('TopAbs_IN') for s in ss) else 'ON' if any(s.endswith('TopAbs_ON') for s in ss) else 'OUT' if all(s.endswith('TopAbs_OUT') for s in ss) else 'UNKNOWN')
        points.append(dict(point=p,states=states))
record['classification']=dict(tolerance_mm=1e-9,point_count=len(points),both_in=[p for p in points if p['states']==['IN','IN']],points=points)
save()
print('CLASSIFICATION',args.model.name,pair,len(points),len(record['classification']['both_in']),flush=True)

# Check every sampled point by a separate solid-angle method. Near-surface
# winding values are preserved as ambiguous, not rounded into an inside result.
outside=[max(s.xmax for s in boxes)+10,max(s.ymax for s in boxes)+10,max(s.zmax for s in boxes)+10]
positions=[p['point'] for p in points]+[outside]
winding=[]
for shape in shapes:
    vertices,indices=shape.tessellate(0.0005,0.05)
    triangles=np.array([v.toTuple() for v in vertices])[np.array(indices)]
    values=[]
    for p in positions:
        t=triangles-np.array(p);a,b,c=t[:,0],t[:,1],t[:,2]
        na,nb,nc=np.linalg.norm(a,axis=1),np.linalg.norm(b,axis=1),np.linalg.norm(c,axis=1)
        numerator=np.einsum('ij,ij->i',a,np.cross(b,c))
        denominator=na*nb*nc+np.einsum('ij,ij->i',a,b)*nc+np.einsum('ij,ij->i',b,c)*na+np.einsum('ij,ij->i',c,a)*nb
        values.append(float(np.sum(2*np.arctan2(numerator,denominator))/(4*math.pi)))
    winding.append(dict(triangles=len(indices),values=values,outside_control=values[-1]))
record['winding']=dict(tolerance_mm=0.0005,components=winding)
record['winding_both_in_indices']=[i for i in range(len(points)) if all(abs(w['values'][i]-1)<1e-5 for w in winding)]
certificates=[]
for i,p in enumerate(points):
    if p['states']!=['IN','IN'] or i not in record['winding_both_in_indices']:continue
    vertex=cq.Vertex.makeVertex(*p['point'])
    distances=[vertex.distance(s) for s in boundaries]
    if min(distances)>1e-6:
        certificates.append(dict(point=p['point'],boundary_distances_mm=distances,winding=[w['values'][i] for w in winding],interior_ball_radius_mm=.8*min(distances)))
        break
record['interior_overlap_certificates']=certificates
common_zero=record['common'].get('done') and record['common'].get('valid') and record['common'].get('volume_mm3')==0
controls=all(abs(w['outside_control'])<1e-5 for w in winding)
if certificates:
    record['status']='overlap'
elif common_zero and controls and not record['classification']['both_in'] and not record['winding_both_in_indices']:
    record['status']='contact_or_separated_bounded'
    record['negative_scope']='valid zero Common; no shared interior found by bounded source seed cubes or independent winding (not exhaustive sampling)'
else:
    record['status']='disputed'
record['phase']='finished';save()
print('VERDICT',args.model.name,pair,record['status'],flush=True)
