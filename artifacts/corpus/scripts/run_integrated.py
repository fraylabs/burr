"""Sequential release corpus runs; adapted from the baseline run_corpus.py."""
import argparse, hashlib, json, os, pathlib, re, signal, subprocess, sys, time
import psutil

parser = argparse.ArgumentParser()
parser.add_argument("--corpus", type=pathlib.Path, required=True)
parser.add_argument("--manifest", type=pathlib.Path)
parser.add_argument("--output", type=pathlib.Path, required=True)
parser.add_argument("--binary", type=pathlib.Path, required=True)
parser.add_argument("--timeout", type=float, default=600)
parser.add_argument("--switchwire-timeout", type=float, help="Cap the pre-existing 0.38 slow Switchwire import separately")
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
binary_sha = hashlib.sha256(args.binary.read_bytes()).hexdigest()
rows = json.loads((args.manifest or args.corpus / "results.json").read_text())
files = [(args.corpus / row["file"], row["sha256"]) for row in rows]
repro_manifest = (args.manifest.parent / "repros-manifest.json") if args.manifest else args.corpus / "repros" / "manifest.json"
repro_hashes = {row["file"]: row["sha256"] for row in json.loads(repro_manifest.read_text())}
actual_repros = {p.name for p in (args.corpus / "repros").glob("*.step")}
if actual_repros != set(repro_hashes):
    raise RuntimeError(f"Repro manifest mismatch: missing {sorted(set(repro_hashes) - actual_repros)}, unlisted {sorted(actual_repros - set(repro_hashes))}")
files += [(args.corpus / "repros" / name, sha) for name, sha in sorted(repro_hashes.items())]
for file, expected_sha in files:
    sha = hashlib.sha256(file.read_bytes()).hexdigest()
    if sha != expected_sha:
        raise RuntimeError(f"Corpus SHA-256 mismatch: {file.name}")
    timeout_limit = (args.switchwire_timeout if args.switchwire_timeout is not None
                     and file.name.startswith("Voron-Switchwire__") else args.timeout)
    prefix = args.output / file.name
    metrics = pathlib.Path(str(prefix) + ".burr.metrics.json")
    out = pathlib.Path(str(prefix) + ".burr.json")
    if metrics.exists():
        existing = json.loads(metrics.read_text())
        if existing.get("sha256") != sha or existing.get("binary_sha256") != binary_sha:
            raise RuntimeError(f"Recorded run has different source or binary: {file.name}")
        if existing.get("timeout_limit_s") != timeout_limit:
            raise RuntimeError(f"Recorded run has a different measurement cap: {file.name}")
        try:
            recorded_result = out.read_text()
            if existing.get("returncode") == 0:
                json.loads(recorded_result)
        except (OSError, UnicodeError, json.JSONDecodeError):
            pass  # Recover a missing/unreadable result using the same inputs.
        else:
            continue
    print("START", file.name, flush=True)
    begin = time.monotonic()
    peak = 0
    cap = None
    err = pathlib.Path(str(prefix) + ".burr.stderr")
    with out.open("w") as stdout, err.open("w") as stderr:
        env = os.environ.copy()
        env["TRUCK_FACE_DIAG_JSONL"] = str(prefix) + ".diagnostics.jsonl"
        process = subprocess.Popen([str(args.binary), str(file.resolve())], stdout=stdout, stderr=stderr, env=env)
        observed = psutil.Process(process.pid)
        while True:
            done, status, usage = os.wait4(process.pid, os.WNOHANG)
            if done:
                process.returncode = os.waitstatus_to_exitcode(status)
                break
            try:
                peak = max(peak, observed.memory_info().rss)
            except psutil.NoSuchProcess:
                pass
            if time.monotonic() - begin > timeout_limit:
                cap = "time"
            elif peak > 9 * 1024 ** 3:
                cap = "memory"
            if cap:
                try:
                    os.kill(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                done, status, usage = os.wait4(process.pid, 0)
                process.returncode = os.waitstatus_to_exitcode(status)
                break
            time.sleep(0.1)
    scale = 1024 ** 2 if sys.platform == "darwin" else 1024
    record = dict(sha256=sha, binary_sha256=binary_sha, elapsed_s=time.monotonic()-begin,
                  peak_rss_mib=usage.ru_maxrss/scale,
                  peak_rss_method="wait4_ru_maxrss", sampled_peak_rss_mib=peak/1024**2,
                  returncode=process.returncode, timeout=bool(cap), cap=cap,
                  timeout_limit_s=timeout_limit)
    if cap and file.name.startswith("Voron-Switchwire__"):
        record["cap_reason"] = "slow, pre-existing in 0.38"
    match = re.search(r"BENCH_LOADED (\{[^\n]*\})", err.read_text(errors="replace"))
    if match:
        record["load_metadata"] = json.loads(match.group(1))
    metrics.write_text(json.dumps(record, indent=2) + "\n")
    print("END", file.name, json.dumps(record), flush=True)
