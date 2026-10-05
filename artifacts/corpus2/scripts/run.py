"""Run held-out measurements one at a time, using the shared Mac lock.

Existing terminal metrics are preserved. Remove only the model's own terminal
metrics to explicitly rerun it, after reconciling any uncertain prior process.
"""
import argparse
import csv
import datetime
import json
import os
import pathlib
import subprocess
import sys
import time


def yield_requested(stop_file, evidence):
    if not stop_file.exists():
        return False
    (evidence/'.driver-paused.json').write_text(json.dumps(dict(
        phase='paused', stop_file=str(stop_file), at=time.time())))
    print('YIELD',str(stop_file),flush=True)
    return True


def run(root, binary, phase, timeout, evidence, lock, limit, stop_file):
    if yield_requested(stop_file,evidence):
        return False
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
            if yield_requested(stop_file,evidence):
                return False
            marker=pathlib.Path(str(prefix)+('.'+mode+'.json' if mode in ('comparison','contacts') else '.'+mode+'.metrics.json'))
            if marker.exists():
                continue
            if mode in ('comparison','contacts') and not pathlib.Path(str(prefix)+'.scene.json').exists():
                continue
            args=[sys.executable,str(scripts/script),'--model',str(model),*extra]
            if mode not in ('comparison','contacts'):
                args += ['--output',str(evidence),'--timeout',str(timeout)]
            print('START',mode,model.name,flush=True)
            while True:
                if yield_requested(stop_file,evidence):
                    return False
                try:
                    lock.mkdir()
                    break
                except FileExistsError:
                    time.sleep(15)
            owner_file=lock/'owner'
            owner_text=f"{os.environ.get('CORPUS_AGENT_ID','dd9facaa-87e0-817c-92a1-1a1cb63b87dd')} {datetime.datetime.now().isoformat()} {mode} {model.name}\n"
            owner_file.write_text(owner_text)
            try:
                if yield_requested(stop_file,evidence):
                    return False
                result=subprocess.run(args)
            finally:
                if owner_file.read_text()!=owner_text:
                    raise RuntimeError('Shared lock owner changed; refusing cleanup')
                owner_file.unlink()
                lock.rmdir()
            print('END',mode,model.name,result.returncode,flush=True)
            if result.returncode:
                raise RuntimeError('Measurement wrapper failed; inspect logs before retrying')
            time.sleep(20)
    return True


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
    parser.add_argument('--stop-file',type=pathlib.Path,help='Yield between jobs or while waiting for a lock; exits 75 when paused')
    args=parser.parse_args()
    if args.phase in ('measure','release') and not args.binary:
        parser.error('--binary is required for measure')
    evidence=(args.evidence or args.root/'logs').resolve()
    evidence.mkdir(parents=True,exist_ok=True)
    stop_file=(args.stop_file or evidence/'STOP_REQUESTED').resolve()
    if args.work_dir:
        os.environ['BURR_CORPUS_WORK_DIR']=str(args.work_dir.resolve())
        os.environ['TMPDIR']=str(args.work_dir.resolve())
    if args.phase in ('compare','contacts'):
        # Serialize comparison orchestration separately from the shared CAD
        # lock, so two snapshots cannot enqueue the same missing result.
        queue=evidence/('.'+args.phase+'-queue.lock')
        queue.parent.mkdir(parents=True,exist_ok=True)
        while True:
            if yield_requested(stop_file,evidence):
                raise SystemExit(75)
            try:
                queue.mkdir()
                break
            except FileExistsError:
                time.sleep(15)
        try:
            completed=run(args.root,args.binary,args.phase,args.timeout,evidence,args.lock,args.limit,stop_file)
        finally:
            queue.rmdir()
    else:
        completed=run(args.root,args.binary,args.phase,args.timeout,evidence,args.lock,args.limit,stop_file)
    if not completed:
        raise SystemExit(75)
