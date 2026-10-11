import pathlib, subprocess, time, os, json, re, urllib.request, urllib.parse, shutil, argparse

parser = argparse.ArgumentParser()
parser.add_argument('--phase', default='before')
parser.add_argument('--all-corpus2', action='store_true')
parser.add_argument('--skip-census', action='store_true')
parser.add_argument('--only', help='model filename substring for a bounded follow-up')
args = parser.parse_args()

root = pathlib.Path(__file__).resolve().parent.parent
models = sorted((root/'artifacts/corpus2/models').iterdir())
targets = ['Framework Laptop 13 CAD', 'FWKNAQ9_G01', 'jubilee__jubilee', '6x_well_plate', 'demo.step', 'OpenArm_2.0', 'OpenArm_Cell', 'Positron', 'tool_template_assembly']
target_models = [m for m in models if any(t in m.name for t in targets)]
census_models = list(target_models)
if not args.all_corpus2: models = target_models
models += sorted((root/'artifacts/corpus/models').iterdir())
models += [root/'artifacts/corpus2/models/FreeCAD-library__Yellow_gearmotor_L.step']
models = list(dict.fromkeys(models))
if args.only:
    models = [m for m in models if args.only in m.name]
    census_models = [m for m in census_models if args.only in m.name]
out = root/'artifacts/face-loss'/args.phase
out.mkdir(parents=True, exist_ok=True)
binary = root/'target/release/burr'
errors = []
if not args.skip_census:
    print('CENSUS',flush=True)
    with (out/'census.jsonl').open('w') as stdout, (out/'census.stderr').open('w') as stderr:
        subprocess.run([str(root/'target/release/examples/face_loss_census'),*[str(m) for m in census_models]], stdout=stdout,stderr=stderr,check=True,timeout=1200)
    subprocess.run(['python3', str(root/'scripts/face-loss-causes.py'),str(out)],check=True)
for model in models:
    print('CHECK', model.name, flush=True)
    cache = out/'cache'
    env = dict(os.environ, BURR_VIEWER_NO_OPEN='1', BURR_VIEWER_PORT='18765', BURR_CACHE_DIR=str(cache))
    with (out/(model.name+'.stderr')).open('w') as stderr, (out/(model.name+'.stdout')).open('w') as stdout:
        process = subprocess.Popen([str(binary),str(model.parent)],env=env,stdout=stdout,stderr=stderr)
        try:
            for attempt in range(120):
                try:
                    urllib.request.urlopen('http://127.0.0.1:18765/api/health', timeout=1).read()
                    break
                except OSError: time.sleep(.25)
            start = time.monotonic()
            url = 'http://127.0.0.1:18765/api/checks?'+urllib.parse.urlencode({'path':model.name}, quote_via=urllib.parse.quote)
            report = json.loads(urllib.request.urlopen(url,timeout=1200).read())
            elapsed = time.monotonic()-start
            (out/(model.name+'.json')).write_text(json.dumps({'seconds':elapsed,'report':report},indent=2))
            print(model.name, elapsed, report.get('outcome'), report.get('incomplete_reasons'),flush=True)
        except Exception as error:
            errors.append(model.name)
            print('ERROR',model.name,repr(error),flush=True)
            (out/(model.name+'.error')).write_text(repr(error))
            if hasattr(error, 'read'):
                (out/(model.name+'.error-body')).write_bytes(error.read())
        finally:
            process.terminate()
            try: process.wait(timeout=10)
            except subprocess.TimeoutExpired: process.kill(); process.wait()
            shutil.rmtree(cache,ignore_errors=True)
subprocess.run(['python3', str(root/'scripts/face-loss-causes.py'),str(out)],check=True)
if errors:
    raise SystemExit('Missing reports: '+', '.join(errors))
