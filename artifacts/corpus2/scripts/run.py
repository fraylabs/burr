"""Run held-out measurements one at a time, using the shared Mac lock.

Existing terminal metrics are preserved. Remove only the model's own terminal
metrics to explicitly rerun it, after reconciling any uncertain prior process.
"""
import argparse
import csv
import pathlib
import subprocess
import sys
import time


def run(root, binary, phase, timeout, evidence, lock, limit):
    scripts=pathlib.Path(__file__).resolve().parent
    root=root.resolve()
    rows=list(csv.DictReader((root/'sources.csv').open(newline='')))
    for row in rows[:limit]:
        model=root/row['file']
        prefix=evidence/model.name
        if phase in ('measure','release'):
            jobs=[('burr','measure_release.py',['--binary',str(binary.resolve())])]
            if phase=='measure':jobs.insert(0,('occt','measure_reference.py',[]))
        else:
            jobs=[('contacts' if phase=='contacts' else 'comparison','measure_comparison.py',['--timeout',str(timeout),'--evidence',str(evidence),'--corpus-scripts',str(scripts.parents[1]/'corpus/scripts'),*(['--contacts'] if phase=='contacts' else [])])]
        for mode,script,extra in jobs:
            marker=pathlib.Path(str(prefix)+('.'+mode+'.json' if mode in ('comparison','contacts') else '.'+mode+'.metrics.json'))
            if marker.exists():
                continue
            if mode in ('comparison','contacts') and not pathlib.Path(str(prefix)+'.scene.json').exists():
                continue
            args=[sys.executable,str(scripts/script),'--model',str(model),*extra]
            if mode not in ('comparison','contacts'):
                args += ['--output',str(evidence),'--timeout',str(timeout)]
            print('START',mode,model.name,flush=True)
            wrapper="corpus_lock=$1; shift; until mkdir \"$corpus_lock\" 2>/dev/null; do sleep 15; done; printf '%s %s %s\\n' \"${CORPUS_AGENT_ID:-dd9facaa-87e0-817c-92a1-1a1cb63b87dd}\" \"$(date)\" \"$*\" > \"$corpus_lock/owner\"; trap 'rm -r \"$corpus_lock\"' EXIT; \"$@\""
            result=subprocess.run(['zsh','-c',wrapper,'corpus2-job',str(lock),*args])
            print('END',mode,model.name,result.returncode,flush=True)
            if result.returncode:
                raise RuntimeError('Measurement wrapper failed; inspect logs before retrying')
            time.sleep(20)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase',choices=['measure','release','compare','contacts'])
    parser.add_argument('--root',type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1])
    parser.add_argument('--binary',type=pathlib.Path)
    parser.add_argument('--timeout',type=float,default=600)
    parser.add_argument('--evidence',type=pathlib.Path)
    parser.add_argument('--lock',type=pathlib.Path,default=pathlib.Path.home()/'coding/fray/.fray/burr/build.lock')
    parser.add_argument('--limit',type=int)
    parser.add_argument('--work-dir',type=pathlib.Path)
    args=parser.parse_args()
    if args.phase in ('measure','release') and not args.binary:
        parser.error('--binary is required for measure')
    evidence=(args.evidence or args.root/'logs').resolve()
    evidence.mkdir(parents=True,exist_ok=True)
    if args.work_dir:
        import os
        os.environ['BURR_CORPUS_WORK_DIR']=str(args.work_dir.resolve())
        os.environ['TMPDIR']=str(args.work_dir.resolve())
    if args.phase in ('compare','contacts'):
        # Serialize comparison orchestration separately from the shared CAD
        # lock, so two snapshots cannot enqueue the same missing result.
        queue=evidence/('.'+args.phase+'-queue.lock')
        queue.parent.mkdir(parents=True,exist_ok=True)
        while True:
            try:
                queue.mkdir()
                break
            except FileExistsError:
                time.sleep(15)
        try:
            run(args.root,args.binary,args.phase,args.timeout,evidence,args.lock,args.limit)
        finally:
            queue.rmdir()
    else:
        run(args.root,args.binary,args.phase,args.timeout,evidence,args.lock,args.limit)
