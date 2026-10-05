"""Independently check every Burr contact-or-separated pair for exactly zero Common.

Uses the existing strict occurrence identity, placement and surface matcher.
No volume threshold turns a nonzero contact into a zero. Refusals and partial
results remain unverified. Run under the shared CAD lock with a resource cap.
"""
import argparse
import json
import pathlib
import sys


def verify(model, evidence, scripts):
    prefix = evidence / model.name
    destination = pathlib.Path(str(prefix) + '.contacts.json')
    report = json.loads(pathlib.Path(str(prefix) + '.burr.json').read_text())['report']
    contacts = report.get('contact_pairs', [])
    result = dict(model=model.name, reported_count=len(contacts), complete=False,
                  checked=[], nonzero_pairs=[], refused_pairs=[])

    def save():
        destination.write_text(json.dumps(result, indent=2) + '\n')

    save()
    if not contacts:
        result['complete'] = True
        save()
        return
    try:
        sys.path.insert(0, str(scripts))
        import cadquery as cq
        from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
        from OCP.BRepCheck import BRepCheck_Analyzer
        from compare_pairs import compare
        from occt_components import components
        exact = components(model)
        if len(exact) == 1 and len(cq.Shape.cast(exact[0][1]).Solids()) > 1:
            exact = [('solid:' + str(i), s.wrapped)
                     for i, s in enumerate(cq.Shape.cast(exact[0][1]).Solids())]
        # Only use synthetic findings to select the contact occurrences for
        # strict mapping. Their synthetic comparison is never accuracy evidence.
        mapping_report = pathlib.Path(str(prefix) + '.contact-mapping-report.json')
        mapping_ref = pathlib.Path(str(prefix) + '.contact-mapping-reference.json')
        mapping_report.write_text(json.dumps(dict(report=dict(findings=contacts))))
        mapping_ref.write_text(json.dumps(dict(names=[n for n, _ in exact],
                                               findings=[], pair_check_complete=True)))
        mapped = compare(model, pathlib.Path(str(prefix) + '.scene.json'),
                         mapping_report, mapping_ref, reported_pairs_only=True)
        result['mapping'] = mapped['mapping']
        mapping = {entry['burr']: entry['occt'] for entry in mapped['mapping']}
        shapes = [cq.Shape.cast(shape) for _, shape in exact]
        valid = {}
        for contact in contacts:
            burr_pair = [c['occurrence_index'] for c in contact['components']]
            pair = sorted(mapping[i] for i in burr_pair)
            entry = dict(burr_pair=burr_pair, occt_pair=pair,
                         code=contact['code'], proof=contact)
            try:
                for i in pair:
                    if i not in valid:
                        valid[i] = bool(BRepCheck_Analyzer(shapes[i].wrapped).IsValid())
                if not all(valid[i] for i in pair):
                    raise ValueError('Invalid source component; cannot certify zero')
                common = BRepAlgoAPI_Common(shapes[pair[0]].wrapped, shapes[pair[1]].wrapped)
                common.Build()
                if not common.IsDone() or not BRepCheck_Analyzer(common.Shape()).IsValid():
                    raise ValueError('OCCT Common unfinished or invalid')
                volume = abs(cq.Shape.cast(common.Shape()).Volume())
                entry.update(common_volume_mm3=volume, exactly_zero=volume == 0.0,
                             common_solids=len(cq.Shape.cast(common.Shape()).Solids()))
                result['checked'].append(entry)
                if volume != 0.0:
                    result['nonzero_pairs'].append(entry)
            except Exception as error:
                entry['refused'] = str(error)
                result['refused_pairs'].append(entry)
            save()
            print(model.name, 'contact', pair, entry.get('common_volume_mm3', entry.get('refused')), flush=True)
        result['complete'] = not result['refused_pairs'] and len(result['checked']) == len(contacts)
    except Exception as error:
        result['refused'] = str(error)
    save()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model', type=pathlib.Path, required=True)
    parser.add_argument('--evidence', type=pathlib.Path, required=True)
    parser.add_argument('--corpus-scripts', type=pathlib.Path, required=True)
    args = parser.parse_args()
    verify(args.model, args.evidence, args.corpus_scripts)
