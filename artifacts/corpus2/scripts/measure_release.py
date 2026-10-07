"""Measure the installed Burr through its real HTTP endpoints; no source build.

Run exactly one model per invocation under the shared CAD lock. Requires numpy
and psutil from the existing corpus OCCT environment. Each invocation starts a
fresh Burr process and an empty persistent viewer cache. Evidence stays local.
"""
import argparse
import concurrent.futures
import json
import os
import pathlib
import re
import shutil
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request

import numpy as np
import psutil


def request(base, route, timeout):
    try:
        with urllib.request.urlopen(base + route, timeout=timeout) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def measure(binary, model, output, timeout):
    output.mkdir(parents=True, exist_ok=True)
    prefix = output / model.name
    cache = pathlib.Path(os.environ.get('BURR_CORPUS_WORK_DIR', str(output / '.work'))) / 'cache'
    if cache.exists():
        shutil.rmtree(cache)
    env = dict(os.environ, BURR_VIEWER_NO_OPEN='1', BURR_CACHE_DIR=str(cache),
               TRUCK_FACE_DIAG_JSONL=str(prefix) + '.diagnostics.jsonl')
    start = time.monotonic()
    with open(str(prefix) + '.server.stdout', 'w') as stdout, open(str(prefix) + '.server.stderr', 'w') as stderr:
        process = subprocess.Popen([str(binary), str(model.parent)], env=env, stdout=stdout, stderr=stderr)
        proc = psutil.Process(process.pid)
        metrics = dict(binary=str(binary), timeout_s=timeout, cache='cold', sampled_peak_rss_mib=0)
        try:
            for _ in range(200):
                text = pathlib.Path(str(prefix) + '.server.stdout').read_text()
                match = re.search(r'OPEN (http://127\.0\.0\.1:\d+/)', text)
                if match:
                    break
                if process.poll() is not None:
                    raise RuntimeError('Server exited before announcing a URL')
                time.sleep(.025)
            else:
                raise TimeoutError('Server did not announce a URL')
            base = match.group(1).rstrip('/')
            query = urllib.parse.urlencode(dict(path=model.name, load='corpus2'), quote_via=urllib.parse.quote)
            stages = []
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                import_start = time.monotonic()
                pending = pool.submit(request, base, '/viewer?' + query, timeout)
                while not pending.done():
                    metrics['sampled_peak_rss_mib'] = max(metrics['sampled_peak_rss_mib'], proc.memory_info().rss / 2**20)
                    if metrics['sampled_peak_rss_mib'] > 9*1024:
                        process.kill()
                        metrics['memory_cap_exceeded'] = True
                        raise MemoryError('9 GiB cap exceeded')
                    try:
                        _, status = request(base, '/api/load-status?id=corpus2', 2)
                        status = json.loads(status)
                        if not stages or stages[-1]['status'] != status:
                            stages.append(dict(elapsed_s=time.monotonic()-import_start, status=status))
                    except (OSError, ValueError):
                        pass
                    time.sleep(.05)
                code, html = pending.result()
            metrics.update(viewer_http_status=code, import_and_viewer_s=time.monotonic()-import_start, stages=stages)
            pathlib.Path(str(prefix) + '.viewer.html').write_bytes(html)
            if code != 200:
                metrics['error'] = html.decode(errors='replace')[-2000:]
                return metrics
            # The server's stage boundary gives a sampled upper bound for pure
            # parsing+tessellation; HTTP duration additionally includes viewer work.
            finished_import = next((s['elapsed_s'] for s in stages if s['status']['stage'] in ['Preparing materials', 'Building viewer']), None)
            metrics['import_stage_upper_bound_s'] = finished_import
            check_start = time.monotonic()
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                pending = pool.submit(request, base, '/api/checks?' + urllib.parse.urlencode(dict(path=model.name), quote_via=urllib.parse.quote), timeout)
                while not pending.done():
                    metrics['sampled_peak_rss_mib'] = max(metrics['sampled_peak_rss_mib'], proc.memory_info().rss / 2**20)
                    if metrics['sampled_peak_rss_mib'] > 9*1024:
                        process.kill()
                        metrics['memory_cap_exceeded'] = True
                        raise MemoryError('9 GiB cap exceeded')
                    time.sleep(.05)
                code, raw_report = pending.result()
            metrics.update(check_http_status=code, check_s=time.monotonic()-check_start)
            report = json.loads(raw_report)
            pathlib.Path(str(prefix) + '.burr.json').write_text(json.dumps(dict(report=report), indent=2)+'\n')
            if code != 200:
                metrics['error'] = report
                return metrics
            manifest = json.loads(html.decode().split('const burrManifest = ',1)[1].split(';\n',1)[0])
            pathlib.Path(str(prefix) + '.manifest.json').write_text(json.dumps(manifest))
            names = {}
            for finding in report.get('findings',[]) + report.get('unresolved_pairs',[]) + report.get('contact_pairs',[]):
                for c in finding['components']:
                    names[c['occurrence_index']] = c.get('definition_name', c['name'])
            vertices = []
            for definition in manifest['definitions']:
                code, raw = request(base, '/mesh/' + definition['id'], timeout)
                if code != 200:
                    raise RuntimeError('Mesh download failed')
                expected = definition['vertices']*definition['stride'] + definition['indices']*4
                if len(raw) != expected:
                    raise ValueError('Mesh payload length differs from manifest')
                (output / (definition['id'] + '.mesh')).write_bytes(raw)
                vertices.append(np.frombuffer(raw, dtype='<f4', count=definition['vertices']*definition['stride']//4).reshape((-1,definition['stride']//4))[:,:3])
            parts = []
            for occurrence in manifest['occurrences']:
                v = vertices[occurrence['geometry']]
                matrix = np.array(occurrence['transform'], dtype=np.float32).reshape((4,4), order='F')
                q = v @ matrix[:3,:3].T + matrix[:3,3]
                if not len(q):
                    raise ValueError('Occurrence has no vertices')
                sample = q[::max(1,len(q)//32)][:32]
                parts.append(dict(index=occurrence['id'], name=names.get(occurrence['id'],''), min=q.min(axis=0).tolist(), max=q.max(axis=0).tolist(), sample_points=sample.tolist(), transform=occurrence['transform'], geometry=occurrence['geometry']))
            pathlib.Path(str(prefix) + '.scene.json').write_text(json.dumps(dict(parts=parts, producer='released HTTP viewer mesh; names from check references where available')))
            metrics['component_count'] = report['component_count']
            metrics['outcome'] = report['outcome']
            metrics['lost_faces'] = 0
            for reason in report.get('incomplete_reasons',[]):
                if reason['code'] == 'step_faces_lost':
                    found = re.search(r'(\d+) of (\d+) STEP faces',reason['message'])
                    if found:
                        metrics.update(lost_faces=int(found[1]), declared_faces=int(found[2]))
        except Exception as error:
            metrics.update(error=repr(error), timeout=isinstance(error,(TimeoutError,concurrent.futures.TimeoutError)))
        finally:
            if process.returncode is None:
                process.terminate()
                _, status, usage = os.wait4(process.pid,0)
                process.returncode = os.waitstatus_to_exitcode(status)
                metrics['peak_rss_mib'] = usage.ru_maxrss / 2**20
                metrics['peak_rss_method'] = 'wait4 ru_maxrss bytes macOS; includes import, checking and mesh serving'
                metrics['intentional_server_shutdown'] = True
            else:
                metrics['server_exit_code'] = process.returncode
            metrics['elapsed_s'] = time.monotonic()-start
            pathlib.Path(str(prefix) + '.burr.metrics.json').write_text(json.dumps(metrics,indent=2)+'\n')
    return metrics


if __name__ == '__main__':
    if os.environ.get('BURR_CORPUS_WORK_DIR') and (pathlib.Path(os.environ['BURR_CORPUS_WORK_DIR']) / 'stop-release-jobs').exists():
        raise SystemExit(99)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=pathlib.Path,required=True)
    parser.add_argument('--model',type=pathlib.Path,required=True)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--timeout',type=float,default=600)
    parser.add_argument('--work-dir',type=pathlib.Path)
    args=parser.parse_args()
    if args.work_dir:
        args.work_dir.mkdir(parents=True,exist_ok=True)
        os.environ['BURR_CORPUS_WORK_DIR']=str(args.work_dir.resolve())
        os.environ['TMPDIR']=str(args.work_dir.resolve())
    print(json.dumps(measure(args.binary.resolve(),args.model.resolve(),args.output.resolve(),args.timeout)),flush=True)
