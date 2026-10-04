"""Author a curved solid overlapping a large planar solid by 0.2 mm."""
from pathlib import Path

from OCP.BRep import BRep_Builder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_AsIs, STEPControl_Writer
from OCP.TopoDS import TopoDS_Compound
from OCP.gp import gp_Pnt

cylinder = BRepPrimAPI_MakeCylinder(10, 1).Shape()
plate = BRepPrimAPI_MakeBox(gp_Pnt(9.8, -250, 0.2), 1, 500, 0.6).Shape()
assert all(BRepCheck_Analyzer(s).IsValid() for s in (cylinder, plate))
common = BRepAlgoAPI_Common(cylinder, plate)
common.Build()
assert common.IsDone() and BRepCheck_Analyzer(common.Shape()).IsValid()
properties = GProp_GProps()
BRepGProp.VolumeProperties_s(common.Shape(), properties)
assert properties.Mass() > 0.1
builder = BRep_Builder()
shape = TopoDS_Compound()
builder.MakeCompound(shape)
for solid in (cylinder, plate):
    builder.Add(shape, solid)
writer = STEPControl_Writer()
writer.Transfer(shape, STEPControl_AsIs)
assert writer.Write(str(Path(__file__).with_name('multiscale-planar.step'))) == IFSelect_RetDone
print('exact common volume mm3:', properties.Mass())
