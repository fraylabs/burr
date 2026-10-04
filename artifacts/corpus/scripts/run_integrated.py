"""Sequential release corpus runs; adapted from the baseline run_corpus.py."""
import argparse, hashlib, json, os, pathlib, re, signal, subprocess, sys, time
import psutil

parser = argparse.ArgumentParser()
parser.add_argument("--corpus", type=pathlib.Path, required=True)
parser.add_argument("--manifest", type=pathlib.Path)
parser.add_argument("--output", type=pathlib.Path, required=True)
parser.add_argument("--binary", type=pathlib.Path, required=True)
parser.add_argument("--timeout", type=float, default=600)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
binary_sha = hashlib.sha256(args.binary.read_bytes()).hexdigest()
rows = json.loads((args.manifest or args.corpus / "results.json").read_text())
files = [(args.corpus / row["file"], row["sha256"]) for row in rows]
repro_manifest = (args.manifest.parent / "repros-manifest.json") if args.manifest else args.corpus / "repros" / "manifest.json"
repro_hashes = {row["file"]: row["sha256"] for row in json.loads(repro_manifest.read_text())} if repro_manifest.exists() else {}
files += [(p, repro_hashes.get(p.name)) for p in sorted((args.corpus / "repros").glob("*.step"))]
for file, expected_sha in files:
    sha = hashlib.sha256(file.read_bytes()).hexdigest()
    if expected_sha and sha != expected_sha:
        raise RuntimeError(f"Corpus SHA-256 mismatch: {file.name}")
    prefix = args.output / file.name
    metrics = pathlib.Path(str(prefix) + ".burr.metrics.json")
    if metrics.exists():
        existing = json.loads(metrics.read_text())
        if existing.get("sha256") != sha or existing.get("binary_sha256") != binary_sha:
            raise RuntimeError(f"Recorded run has different source or binary: {file.name}")
        continue
    print("START", file.name, flush=True)
    begin = time.monotonic()
    peak = 0
    cap = None
    out = pathlib.Path(str(prefix) + ".burr.json")
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
            if time.monotonic() - begin > args.timeout:
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
                  timeout_limit_s=args.timeout)
    match = re.search(r"BENCH_LOADED (\{[^\n]*\})", err.read_text(errors="replace"))
    if match:
        record["load_metadata"] = json.loads(match.group(1))
    metrics.write_text(json.dumps(record, indent=2) + "\n")
    print("END", file.name, json.dumps(record), flush=True)
