"""Run the existing strict pair comparator on meshes from the released server.

For witness-only or timed-out references, validate every reported occurrence
before checking exactly its reported pairs in OCCT. The subset result cannot
certify recall or a clean pass. No matching or volume tolerance is changed.
"""
import argparse
import json
import pathlib
import sys


def run(model, evidence, scripts):
    sys.path.insert(0, str(scripts))
    import cadquery as cq
    from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
    from OCP.BRepCheck import BRepCheck_Analyzer
    from compare_pairs import compare
    from occt_components import components
    prefix = evidence / model.name
    scene = pathlib.Path(str(prefix)+'.scene.json')
    burr = pathlib.Path(str(prefix)+'.burr.json')
    reference_path = pathlib.Path(str(prefix)+'.occt.json')
    comparison_path = pathlib.Path(str(prefix)+'.comparison.json')
    try:
        if reference_path.exists():
            ref = json.loads(reference_path.read_text())
        else:
            metrics = json.loads(pathlib.Path(str(prefix)+'.occt.metrics.json').read_text())
            ref = dict(metrics['load_metadata'], pair_check_complete=False, findings=[])
            reference_path = pathlib.Path(str(prefix)+'.occt-import-only.json')
            reference_path.write_text(json.dumps(ref))
        if ref.get('pair_check_complete') and not ref.get('errors') and not ref.get('invalid_solids'):
            try:
                result = compare(model, scene, burr, reference_path)
            except Exception as full_error:
                result = compare(model, scene, burr, reference_path, reported_pairs_only=True)
                result['full_comparison_refusal'] = str(full_error)
        else:
            report = json.loads(burr.read_text())['report']
            if not report['findings']:
                raise ValueError('No findings to check against a partial/invalid reference; full comparison unavailable')
            # compare() validates unique placement, identity and source surface
            # samples before returning a mapping. This temporary empty reference
            # is used only to obtain that mapping, never as exact pair evidence.
            temporary = pathlib.Path(str(prefix)+'.mapping-only.json')
            temporary.write_text(json.dumps(dict(names=ref['names'],findings=[],pair_check_complete=True)))
            mapped = compare(model, scene, burr, temporary, reported_pairs_only=True)
            mapping = {entry['burr']:entry['occt'] for entry in mapped['mapping']}
            exact = components(model)
            if len(exact) == 1 and len(cq.Shape.cast(exact[0][1]).Solids()) > 1:
                exact = [('solid:'+str(i),s.wrapped) for i,s in enumerate(cq.Shape.cast(exact[0][1]).Solids())]
            shapes = [cq.Shape.cast(s) for _,s in exact]
            checked = sorted({tuple(sorted(mapping[c['occurrence_index']] for c in f['components'])) for f in report['findings']})
            findings=[]
            for i,j in checked:
                if not all(BRepCheck_Analyzer(shapes[k].wrapped).IsValid() for k in (i,j)):
                    raise ValueError(f'Invalid source solid in reported pair {(i,j)}')
                common=BRepAlgoAPI_Common(shapes[i].wrapped,shapes[j].wrapped)
                common.Build()
                if not common.IsDone() or not BRepCheck_Analyzer(common.Shape()).IsValid():
                    raise ValueError(f'Invalid OCCT Common in reported pair {(i,j)}')
                volume=abs(cq.Shape.cast(common.Shape()).Volume())
                threshold=max(1e-6,min(abs(shapes[i].Volume()),abs(shapes[j].Volume()))*1e-9)
                if volume > threshold:
                    findings.append(dict(pair=[i,j],volume_mm3=volume,threshold_mm3=threshold))
            subset=pathlib.Path(str(prefix)+'.occt-reported.json')
            subset.write_text(json.dumps(dict(names=ref['names'],findings=findings,pair_check_complete=True,pair_check_scope='reported_pairs',checked_pairs=checked),indent=2))
            result=compare(model,scene,burr,subset,reported_pairs_only=True)
        comparison_path.write_text(json.dumps(result,indent=2)+'\n')
        print(model.name,'matched',len(result['matched_pairs']),'extra',len(result['extra_pairs']),'missing',len(result['missing_pairs']),result['comparison_mode'],flush=True)
    except Exception as error:
        comparison_path.write_text(json.dumps(dict(model=model.name,refused=str(error)),indent=2)+'\n')
        print(model.name,'REFUSED',str(error),flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model',type=pathlib.Path,required=True)
    parser.add_argument('--evidence',type=pathlib.Path,required=True)
    parser.add_argument('--corpus-scripts',type=pathlib.Path,required=True)
    args=parser.parse_args()
    run(args.model,args.evidence,args.corpus_scripts)
