import sys, json, pathlib, time, math, itertools
import numpy as np
import cadquery as cq
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common, BRepAlgoAPI_Cut
from OCP.gp import gp_Pnt
import argparse, subprocess
parser=argparse.ArgumentParser(description="Cross-check source-pair interior points using exact classification, mesh winding and bilateral Boolean volumes. Wrap this command in the shared CAD lock.")
parser.add_argument("--model",type=pathlib.Path,required=True)
parser.add_argument("--request",type=pathlib.Path,required=True,help="JSON list: burr_pair, occt_pair, point in source STEP coordinates")
parser.add_argument("--output",type=pathlib.Path,required=True)
parser.add_argument("--corpus-scripts",type=pathlib.Path,required=True)
parser.add_argument("--timeout",type=float,default=150)
parser.add_argument("--worker",action="store_true",help=argparse.SUPPRESS)
args=parser.parse_args()
if not args.worker:
    try:
        code=subprocess.run([sys.executable,str(pathlib.Path(__file__).resolve()),*sys.argv[1:],"--worker"],timeout=args.timeout).returncode
    except subprocess.TimeoutExpired:
        print("Source cross-check hit hard timeout; preserve partial output.",file=sys.stderr)
        code=124
    raise SystemExit(code)
sys.path.insert(0,str(args.corpus_scripts))
from occt_components import components
source=components(args.model)
requests=json.loads(args.request.read_text())
result=[]
def save():
    args.output.write_text(json.dumps(result,indent=2))
def winding(s,points):
    vertices,indices=s.tessellate(0.0005,0.05)
    v=np.array([x.toTuple() for x in vertices]); triangles=v[np.array(indices)]
    values=[]
    for point in points:
        t=triangles-np.array(point);a,b,c=t[:,0],t[:,1],t[:,2]
        na,nb,nc=np.linalg.norm(a,axis=1),np.linalg.norm(b,axis=1),np.linalg.norm(c,axis=1)
        num=np.einsum('ij,ij->i',a,np.cross(b,c))
        den=na*nb*nc+np.einsum('ij,ij->i',a,b)*nc+np.einsum('ij,ij->i',b,c)*na+np.einsum('ij,ij->i',c,a)*nb
        values.append(float(np.sum(2*np.arctan2(num,den))/(4*math.pi)))
    return dict(tessellation_tolerance_mm=0.0005,triangles=len(indices),winding_numbers=values)
for request in requests:
    pair,indices,seed=request['burr_pair'],request['occt_pair'],request['point']
    shapes=[cq.Shape.cast(source[i][1]) for i in indices]
    row=dict(burr_pair=pair,occt_pair=indices,names=[source[i][0] for i in indices],source_valid=[s.isValid() for s in shapes],seed=seed,points=[])
    result.append(row);save();print('START',pair,flush=True)
    classifiers=[BRepClass3d_SolidClassifier(s.Solids()[0].wrapped) for s in shapes]
    boundaries=[cq.Compound.makeCompound(s.Faces()) for s in shapes]
    # The supplied seed is a hypothesis. This independent source import
    # checks a local 3x3x3 cube using exact classification, without any Boolean.
    for offset in itertools.product([-0.002,0,0.002],repeat=3):
        xyz=[a+b for a,b in zip(seed,offset)];states=[]
        for c in classifiers:
            c.Perform(gp_Pnt(*xyz),1e-9);states.append(str(c.State()))
        entry=dict(point=xyz,states=states)
        if offset==(0,0,0):
            v=cq.Vertex.makeVertex(*xyz)
            entry['boundary_distances_mm']=[v.distance(s) for s in boundaries]
        row['points'].append(entry)
    row['both_in_count']=sum(all(s.endswith('TopAbs_IN') for s in p['states']) for p in row['points'])
    save();print('CLASSIFY',pair,row['both_in_count'],flush=True)
    # Solid-angle winding is a separate numerical algorithm, not BRepClass3d
    # ray classification or the BOP Common/Cut engine. Outside controls are kept.
    box=shapes[0].BoundingBox()
    outside=[box.xmax+10,box.ymax+10,box.zmax+10]
    sample=[seed,row['points'][0]['point'],row['points'][-1]['point'],outside]
    row['winding_sample_points']=sample
    row['winding']=[winding(s,sample) for s in shapes]
    save();print('WINDING',pair,row['winding'],flush=True)
    a,b=shapes
    row['booleans']={}
    for name,cls,x,y in [('common',BRepAlgoAPI_Common,a,b),('cut_a_b',BRepAlgoAPI_Cut,a,b),('cut_b_a',BRepAlgoAPI_Cut,b,a)]:
        operation=cls(x.wrapped,y.wrapped);operation.SetRunParallel(False);operation.SetNonDestructive(True);operation.Build()
        data=dict(done=operation.IsDone())
        if operation.IsDone():
            s=cq.Shape.cast(operation.Shape());data.update(valid=s.isValid(),volume_mm3=s.Volume(),solid_count=len(s.Solids()))
            if name.startswith('cut'):data['volume_loss_mm3']=x.Volume()-s.Volume()
        row['booleans'][name]=data
        save()
    print('END',pair,row['booleans'],flush=True)
