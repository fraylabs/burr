"""Bound a strict OCCT pair comparison without converting a timeout to success."""
import argparse
import json
import os
import pathlib
import subprocess
import sys
import time
import psutil


def measure(model,evidence,scripts,timeout):
    prefix=evidence/model.name
    command=[sys.executable,str(pathlib.Path(__file__).with_name('compare_release.py')),
             '--model',str(model),'--evidence',str(evidence),'--corpus-scripts',str(scripts)]
    start=time.monotonic()
    process=subprocess.Popen(command)
    proc=psutil.Process(process.pid);peak=0;cap=None
    while True:
        done,status,usage=os.wait4(process.pid,os.WNOHANG)
        if done:break
        try:peak=max(peak,proc.memory_info().rss)
        except psutil.NoSuchProcess:pass
        if time.monotonic()-start > timeout or peak > 9*1024**3:
            cap='timeout' if time.monotonic()-start > timeout else 'memory'
            process.kill();done,status,usage=os.wait4(process.pid,0)
            break
        time.sleep(.1)
    metrics=dict(elapsed_s=time.monotonic()-start,peak_rss_mib=usage.ru_maxrss/2**20,
                 returncode=os.waitstatus_to_exitcode(status),cap=cap,timeout_s=timeout,
                 peak_rss_method='wait4 ru_maxrss bytes macOS')
    pathlib.Path(str(prefix)+'.comparison.metrics.json').write_text(json.dumps(metrics,indent=2)+'\n')
    result=pathlib.Path(str(prefix)+'.comparison.json')
    if cap or not result.exists():
        result.write_text(json.dumps(dict(model=model.name,refused=f'Comparison {cap or "process failure"}; no complete pair result'),indent=2)+'\n')
    process.returncode=metrics['returncode']
    print(model.name,'comparison resources',metrics,flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model',type=pathlib.Path,required=True)
    parser.add_argument('--evidence',type=pathlib.Path,required=True)
    parser.add_argument('--corpus-scripts',type=pathlib.Path,required=True)
    parser.add_argument('--timeout',type=float,default=600)
    args=parser.parse_args()
    measure(args.model,args.evidence,args.corpus_scripts,args.timeout)
