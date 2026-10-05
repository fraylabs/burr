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


def run(root, binary, phase, timeout):
    scripts=pathlib.Path(__file__).resolve().parent
    root=root.resolve()
    for row in csv.DictReader((root/'sources.csv').open(newline='')):
        model=root/row['file']
        prefix=root/'logs'/model.name
        if phase=='measure':
            jobs=[('occt','measure_reference.py',[]),('burr','measure_release.py',['--binary',str(binary.resolve())])]
        else:
            jobs=[('comparison','measure_comparison.py',['--timeout',str(timeout),'--evidence',str(root/'logs'),'--corpus-scripts',str(scripts.parents[1]/'corpus/scripts')])]
        for mode,script,extra in jobs:
            marker=pathlib.Path(str(prefix)+('.comparison.json' if mode=='comparison' else '.'+mode+'.metrics.json'))
            if marker.exists():
                continue
            if mode=='comparison' and not pathlib.Path(str(prefix)+'.scene.json').exists():
                continue
            args=[sys.executable,str(scripts/script),'--model',str(model),*extra]
            if mode!='comparison':
                args += ['--output',str(root/'logs'),'--timeout',str(timeout)]
            print('START',mode,model.name,flush=True)
            result=subprocess.run(['zsh','-c','until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done; trap "rmdir /tmp/burr-build.lock" EXIT; "$@"','corpus2-job',*args])
            print('END',mode,model.name,result.returncode,flush=True)
            if result.returncode:
                raise RuntimeError('Measurement wrapper failed; inspect logs before retrying')
            time.sleep(20)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase',choices=['measure','compare'])
    parser.add_argument('--root',type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1])
    parser.add_argument('--binary',type=pathlib.Path)
    parser.add_argument('--timeout',type=float,default=600)
    args=parser.parse_args()
    if args.phase=='measure' and not args.binary:
        parser.error('--binary is required for measure')
    if args.phase=='compare':
        # Serialize comparison orchestration separately from the shared CAD
        # lock, so two snapshots cannot enqueue the same missing result.
        queue=pathlib.Path('/tmp/burr-corpus2/comparison-queue.lock')
        queue.parent.mkdir(parents=True,exist_ok=True)
        while True:
            try:
                queue.mkdir()
                break
            except FileExistsError:
                time.sleep(15)
        try:
            run(args.root,args.binary,args.phase,args.timeout)
        finally:
            queue.rmdir()
    else:
        run(args.root,args.binary,args.phase,args.timeout)
