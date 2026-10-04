from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut,BRepAlgoAPI_Common
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.gp import gp_Trsf,gp_Ax1,gp_Pnt,gp_Dir
from OCP.BRep import BRep_Builder
from OCP.TopoDS import TopoDS_Compound
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.STEPControl import STEPControl_Writer,STEPControl_AsIs
from OCP.IFSelect import IFSelect_RetDone
import math
ring=BRepAlgoAPI_Cut(BRepPrimAPI_MakeCylinder(14,10).Shape(),BRepPrimAPI_MakeCylinder(10,10).Shape()).Shape()
disk=BRepPrimAPI_MakeCylinder(10,10).Shape();t=gp_Trsf();t.SetRotation(gp_Ax1(gp_Pnt(0,0,0),gp_Dir(0,0,1)),math.pi/24);disk=BRepBuilderAPI_Transform(disk,t,True).Shape()
assert BRepCheck_Analyzer(ring).IsValid() and BRepCheck_Analyzer(disk).IsValid()
common=BRepAlgoAPI_Common(ring,disk).Shape();properties=GProp_GProps();BRepGProp.VolumeProperties_s(common,properties);assert abs(properties.Mass())<1e-8
b=BRep_Builder();shape=TopoDS_Compound();b.MakeCompound(shape);b.Add(shape,ring);b.Add(shape,disk)
w=STEPControl_Writer();w.Transfer(shape,STEPControl_AsIs);assert w.Write(str(__import__('pathlib').Path(__file__).with_name('curved-contact.step')))==IFSelect_RetDone
