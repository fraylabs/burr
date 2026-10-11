"""Reduce a STEP assembly to named leaves without re-exporting its geometry.

Retains ancestor occurrences, representation links and referenced source
entities. Always verify the result independently in OCCT and released Burr.
The original Latin-1 header is preserved; retain source licence attribution.
"""
import argparse
import pathlib
import re


def references(entity):
    return [int(x) for x in re.findall(r'#(\d+)', re.sub(r"'(?:[^']|'')*'", "''", entity))]


def entity_type(entity,name):
    return re.match(re.escape(name)+r"\s*\(",entity) is not None


def reduce(source, output, names):
    text=source.read_text(encoding='latin1')
    data=text.split('DATA;',1)[1].rsplit('ENDSEC;',1)[0]
    entities={int(m[1]):m[2].strip() for m in re.finditer(r"#(\d+)\s*=\s*((?:[^;']|'(?:[^']|'')*')*);",data,re.S)}
    products={n for n,s in entities.items() if entity_type(s,'PRODUCT') and any(re.match(r"PRODUCT\s*\(\s*'"+re.escape(name)+r"'",s) for name in names)}
    formations={n for n,s in entities.items() if s.startswith('PRODUCT_DEFINITION_FORMATION') and set(references(s)) & products}
    definitions={n for n,s in entities.items() if entity_type(s,'PRODUCT_DEFINITION') and set(references(s)) & formations}
    if len(definitions) != len(names):
        raise ValueError(f'Names did not select unique definitions: {definitions}')
    occurrences={n:references(s)[-2:] for n,s in entities.items() if entity_type(s,'NEXT_ASSEMBLY_USAGE_OCCURRENCE')}
    kept=set(definitions)
    while True:
        parents={parent for parent,child in occurrences.values() if child in kept}
        if parents <= kept:
            break
        kept |= parents
    selected_occurrences={n for n,(parent,child) in occurrences.items() if child in kept}
    shape_definitions={n for n,s in entities.items() if entity_type(s,'PRODUCT_DEFINITION_SHAPE') and references(s)[-1] in kept | selected_occurrences}
    shape_links={n for n,s in entities.items() if any(entity_type(s,t) for t in ('SHAPE_DEFINITION_REPRESENTATION','CONTEXT_DEPENDENT_SHAPE_REPRESENTATION')) and set(references(s)) & shape_definitions}
    seen=set()
    todo=list(kept | selected_occurrences | shape_definitions | shape_links)
    while True:
        while todo:
            n=todo.pop()
            if n in seen or n not in entities:
                continue
            seen.add(n)
            todo.extend(references(entities[n]))
        inverse={n for n,s in entities.items() if entity_type(s,'SHAPE_REPRESENTATION_RELATIONSHIP') and set(references(s)) & seen} - seen
        if not inverse:
            break
        todo.extend(inverse)
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(text.split('DATA;',1)[0]+'DATA;\n'+'\n'.join(f'#{n}={entities[n]};' for n in sorted(seen))+'\nENDSEC;\nEND-ISO-10303-21;\n',encoding='latin1')
    print(f'{output}: {len(seen)} entities, {output.stat().st_size} bytes; selected definitions {sorted(definitions)}')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=pathlib.Path)
    parser.add_argument('output',type=pathlib.Path)
    parser.add_argument('--keep-name',action='append',required=True)
    args=parser.parse_args()
    reduce(args.source,args.output,args.keep_name)
