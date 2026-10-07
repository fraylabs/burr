"""Count declared STEP assembly leaves without importing or tessellating CAD.

This audits selection size only. Counts include wireframe/empty definitions;
OCCT's geometry census remains authoritative for measured component counts.
Invoke under the shared lock on the shared Mac.
"""
import argparse
import collections
import csv
import json
import mmap
import pathlib
import re


def census(path):
    graph=collections.defaultdict(list)
    with path.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as data:
        for match in re.finditer(rb'#\d+\s*=\s*NEXT_ASSEMBLY_USAGE_OCCURRENCE\s*\((.*?)\)\s*;',data,re.S):
            args=re.sub(rb"'(?:[^']|'')*'",b"''",match[1])
            refs=[int(n) for n in re.findall(rb'#(\d+)',args)]
            if len(refs)!=2:
                raise ValueError('Unexpected NAUO references')
            parent,child=refs
            graph[parent].append(child)
    children={n for group in graph.values() for n in group}
    roots=set(graph)-children
    memo={}
    def leaves(n,active):
        if n in active:
            raise ValueError('Assembly graph cycle')
        if n not in memo:
            memo[n]=sum(leaves(child,active|{n}) for child in graph[n]) if n in graph else 1
        return memo[n]
    if graph and not roots:
        raise ValueError('Assembly graph has no root')
    return dict(declared_usage_records=sum(map(len,graph.values())),root_definitions=sorted(roots),
                expanded_declared_leaves=sum(leaves(n,set()) for n in roots) if graph else None,
                scope='declared assembly graph; geometry not validated')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1])
    args=parser.parse_args();root=args.root.resolve()
    results=[]
    for row in csv.DictReader((root/'sources.csv').open()):
        result=dict(file=row['file'],**census(root/row['file']))
        results.append(result);print(result,flush=True)
    (root/'declared-census.json').write_text(json.dumps(results,indent=2)+'\n')
