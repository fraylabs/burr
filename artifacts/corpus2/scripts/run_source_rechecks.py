"""Run capped source-pair rechecks one at a time with ownership receipts."""
import argparse
import datetime
import json
import os
import pathlib
import subprocess
import threading
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--jobs", type=pathlib.Path, required=True, help="JSON list of [model filename, [Burr index, Burr index]]")
parser.add_argument("--models", type=pathlib.Path, required=True)
parser.add_argument("--evidence", type=pathlib.Path, required=True)
parser.add_argument("--output", type=pathlib.Path, required=True)
parser.add_argument("--corpus-scripts", type=pathlib.Path, required=True)
parser.add_argument("--python", type=pathlib.Path, required=True)
parser.add_argument("--work-dir", type=pathlib.Path, required=True)
parser.add_argument("--lock", type=pathlib.Path, required=True)
parser.add_argument("--owner", required=True)
parser.add_argument("--list", choices=["contact_pairs", "findings"], default="contact_pairs")
parser.add_argument("--timeout", type=float, default=150)
parser.add_argument("--stop-file", type=pathlib.Path, help="Yield before the next reference job when this file exists")
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
jobs = json.loads(args.jobs.read_text())
(args.output / "jobs.json").write_text(json.dumps(jobs, indent=2))
state = dict(pid=os.getpid(), phase="starting")
stop_file = args.stop_file or args.output / "STOP_REQUESTED"


def yield_requested():
    if not stop_file.exists():
        return False
    state.update(phase="paused")
    (args.output / "paused.json").write_text(json.dumps(dict(state, stop_file=str(stop_file))))
    print("Yield requested; no further reference jobs started", flush=True)
    return True


def heartbeat():
    while True:
        destination = args.output / "driver-state.json"
        temporary = args.output / "driver-state.tmp"
        temporary.write_text(json.dumps(dict(state, heartbeat=time.time())))
        temporary.replace(destination)
        time.sleep(5)


threading.Thread(target=heartbeat, daemon=True).start()
for model, pair in jobs:
    if yield_requested():
        break
    receipt = args.output / (model + "." + "-".join(map(str, pair)) + ".json")
    if receipt.exists() and json.loads(receipt.read_text()).get("phase") == "finished":
        continue
    state.update(phase="waiting", model=model, pair=pair)
    while True:
        if yield_requested():
            break
        try:
            args.lock.mkdir()
            break
        except FileExistsError:
            time.sleep(15)
    if state["phase"] == "paused":
        break
    owner_file = args.lock / "owner"
    owner_text = f"{args.owner} {datetime.datetime.now().isoformat()} source recheck {model} {pair}\n"
    owner_file.write_text(owner_text)
    try:
        if yield_requested():
            break
        state.update(phase="running")
        command = [str(args.python), str(pathlib.Path(__file__).with_name("recheck_source_pair.py")), "--model", str(args.models / model), "--pair", *map(str, pair), "--output", str(receipt), "--evidence", str(args.evidence), "--corpus-scripts", str(args.corpus_scripts), "--list", args.list, "--timeout", str(args.timeout)]
        with receipt.with_suffix(".log").open("a") as log:
            code = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, TMPDIR=str(args.work_dir))).returncode
    finally:
        # Only remove our unchanged ownership receipt and empty directory.
        if owner_file.read_text() != owner_text:
            raise RuntimeError("Shared lock ownership changed; refusing cleanup")
        owner_file.unlink()
        args.lock.rmdir()
    data = json.loads(receipt.read_text()) if receipt.exists() else dict(model=model, burr_pair=pair)
    if code != 0:
        data.update(status="disputed", phase="finished", error=f"Reference returncode {code}")
        receipt.write_text(json.dumps(data, indent=2))
    print(model, pair, data.get("status"), flush=True)
    if data.get("status") == "overlap":
        (args.output / "URGENT-overlap.json").write_text(json.dumps(data, indent=2))
        state.update(phase="overlap_found")
        break
    state.update(phase="gap")
    time.sleep(20)
else:
    state.update(phase="finished")
    (args.output / "complete.json").write_text(json.dumps(dict(jobs=len(jobs))))
(args.output / "driver-state.json").write_text(json.dumps(dict(state, heartbeat=time.time())))
