"""Run released burr . and render one model in installed Google Chrome.

Invoke under the shared lock. Requires Node and a local Playwright module.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import time
import urllib.parse


def render(binary,model,output,playwright):
    output.mkdir(parents=True,exist_ok=True)
    prefix=output/model.name
    # The outer shared CAD lock serializes this check with every capture.
    # A second orchestrator may have queued before the first wrote its marker.
    marker=pathlib.Path(str(prefix)+'.chrome.json')
    if marker.exists():
        previous=json.loads(marker.read_text())
        if previous.get('httpStatus')==200 and previous.get('geometry'):
            print(model.name,'already captured',flush=True)
            return
    cache=pathlib.Path(os.environ.get('BURR_CORPUS_WORK_DIR', str(output / '.work'))) / 'render-cache'
    env=dict(os.environ,BURR_VIEWER_NO_OPEN='1',BURR_CACHE_DIR=str(cache))
    with open(str(prefix)+'.server.stdout','w') as stdout,open(str(prefix)+'.server.stderr','w') as stderr:
        process=subprocess.Popen([str(binary.resolve()),'.'],cwd=model.parent,env=env,stdout=stdout,stderr=stderr)
        try:
            for _ in range(200):
                text=pathlib.Path(str(prefix)+'.server.stdout').read_text()
                match=re.search(r'OPEN (http://127\.0\.0\.1:\d+/)',text)
                if match:
                    break
                if process.poll() is not None:
                    raise RuntimeError('Burr exited before announcing URL')
                time.sleep(.025)
            else:
                raise RuntimeError('Burr did not announce URL')
            url=match[1]+'viewer?'+urllib.parse.urlencode(dict(path=model.name),quote_via=urllib.parse.quote)
            command=['node',str(pathlib.Path(__file__).with_name('render_chrome.mjs')),url,str(prefix),str(playwright.resolve())]
            result=subprocess.run(command,env=env,timeout=660,capture_output=True,text=True)
            pathlib.Path(str(prefix)+'.driver.stdout').write_text(result.stdout)
            pathlib.Path(str(prefix)+'.driver.stderr').write_text(result.stderr)
            if result.returncode:
                raise RuntimeError('Chrome capture failed; see driver stderr')
            print(model.name,'captured',flush=True)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=pathlib.Path,required=True)
    parser.add_argument('--model',type=pathlib.Path,required=True)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--playwright',type=pathlib.Path,required=True)
    args=parser.parse_args()
    render(args.binary,args.model.resolve(),args.output.resolve(),args.playwright)
