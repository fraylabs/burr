# Import leaf component occurrences with their full assembly placement.
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.XCAFDoc import XCAFDoc_DocumentTool
from OCP.TDF import TDF_LabelSequence,TDF_Label
from OCP.TDataStd import TDataStd_Name
from OCP.TopLoc import TopLoc_Location

def components(path):
 doc=TDocStd_Document(TCollection_ExtendedString('corpus'))
 reader=STEPCAFControl_Reader();reader.ReadFile(str(path));reader.Transfer(doc)
 tool=XCAFDoc_DocumentTool.ShapeTool_s(doc.Main());roots=TDF_LabelSequence();tool.GetFreeShapes(roots)
 out=[]
 def name(label):
  n=TDataStd_Name()
  return n.Get().ToExtString() if label.FindAttribute(TDataStd_Name.GetID_s(),n) else ''
 def walk(label,parent,names):
  loc=parent.Multiplied(tool.GetLocation_s(label));ref=TDF_Label()
  definition=ref if tool.GetReferredShape_s(label,ref) else label
  children=TDF_LabelSequence()
  if tool.GetComponents_s(definition,children,False):
   for k in range(1,children.Length()+1):walk(children.Value(k),loc,names+[name(definition) or name(label)])
  else:
   shape=tool.GetShape_s(definition)
   if not shape.IsNull():out.append((('/'.join(names+[name(definition) or name(label)])).strip('/'),shape.Moved(loc)))
 for k in range(1,roots.Length()+1):walk(roots.Value(k),TopLoc_Location(),[])
 return out
if __name__=='__main__':
 import sys,json
 from OCP.BRepCheck import BRepCheck_Analyzer
 c=components(sys.argv[1]);print(json.dumps([dict(name=n,valid=BRepCheck_Analyzer(s).IsValid()) for n,s in c]))
