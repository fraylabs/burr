"""Retain one source STEP face and its dependencies in a new open shell.

This is an importer repro, not an interference reference: it intentionally
contains an open surface. Verify the same diagnostic in the released binary.
Original entity IDs, geometry and header are preserved. Retain source credit.
"""
import argparse
import pathlib
import re
from reduce_pair import references


def reduce(source,output,face):
    text=source.read_text(encoding='latin1')
    data=text.split('DATA;',1)[1].rsplit('ENDSEC;',1)[0]
    entities={int(m[1]):m[2].strip() for m in re.finditer(r"#(\d+)\s*=\s*((?:[^;']|'(?:[^']|'')*')*);",data,re.S)}
    if not entities.get(face,'').startswith(('ADVANCED_FACE(','FACE_SURFACE(')):
        raise ValueError('Selected entity is not a source face')
    todo=[face];seen=set()
    while todo:
        n=todo.pop()
        if n in seen:
            continue
        if n not in entities:
            raise ValueError(f'Missing dependency #{n}')
        seen.add(n);todo.extend(references(entities[n]))
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(text.split('DATA;',1)[0]+'DATA;\n'+'\n'.join(f'#{n}={entities[n]};' for n in sorted(seen))+f"\n#{max(entities)+1}=OPEN_SHELL('',(#{face}));\nENDSEC;\nEND-ISO-10303-21;\n",encoding='latin1')
    print(output,output.stat().st_size,'bytes',len(seen),'source entities')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=pathlib.Path)
    parser.add_argument('output',type=pathlib.Path)
    parser.add_argument('--face',type=int,required=True)
    args=parser.parse_args()
    reduce(args.source,args.output,args.face)
