"""Measure the existing OCCT reference in one process, with a 600 s/9 GiB cap.

Uses macOS wait4 RSS bytes. Records import metadata even if the pair scan does
not complete. Invoke under the shared lock, once per model.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import time

import psutil


def measure(model, output, timeout):
    output.mkdir(parents=True,exist_ok=True)
    prefix=output/model.name
    script=pathlib.Path(__file__).with_name('occt_reference.py')
    start=time.monotonic()
    with open(str(prefix)+'.occt.stdout','w') as out,open(str(prefix)+'.occt.stderr','w') as err:
        process=subprocess.Popen([sys.executable,str(script),str(model)],stdout=out,stderr=err)
        proc=psutil.Process(process.pid)
        peak=0
        capped=None
        while True:
            done,status,usage=os.wait4(process.pid,os.WNOHANG)
            if done:
                break
            try:
                peak=max(peak,proc.memory_info().rss)
            except psutil.NoSuchProcess:
                pass
            if time.monotonic()-start > timeout or peak > 9*1024**3:
                capped='timeout' if time.monotonic()-start > timeout else 'memory'
                process.kill()
                done,status,usage=os.wait4(process.pid,0)
                break
            time.sleep(.1)
        metrics=dict(elapsed_s=time.monotonic()-start,peak_rss_mib=usage.ru_maxrss/2**20,
                     peak_rss_method='wait4 ru_maxrss bytes macOS',returncode=os.waitstatus_to_exitcode(status),
                     timeout=capped is not None,cap=capped,timeout_s=timeout)
        stderr_text=pathlib.Path(str(prefix)+'.occt.stderr').read_text()
        match=re.search(r'OCCT_LOADED (\{[^\n]*\})',stderr_text)
        if match:
            metrics['load_metadata']=json.loads(match[1])
        for line in pathlib.Path(str(prefix)+'.occt.stdout').read_text().splitlines()[::-1]:
            if line.startswith('{'):
                try:
                    reference=json.loads(line)
                    pathlib.Path(str(prefix)+'.occt.json').write_text(json.dumps(reference,indent=2)+'\n')
                    break
                except ValueError:
                    pass
        pathlib.Path(str(prefix)+'.occt.metrics.json').write_text(json.dumps(metrics,indent=2)+'\n')
    return metrics


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model',type=pathlib.Path,required=True)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--timeout',type=float,default=600)
    args=parser.parse_args()
    print(json.dumps(measure(args.model.resolve(),args.output.resolve(),args.timeout)),flush=True)
