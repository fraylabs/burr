#!/usr/bin/env python3
"""Exercise the public installer and HTTP check using only a released binary."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def smoke(version):
    version = re.sub(r"^(burr-v|v)", "", version)
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", version):
        raise ValueError("Expected a published semantic version or burr-v tag")

    with tempfile.TemporaryDirectory(prefix="burr-install-smoke-") as temporary:
        root = Path(temporary)
        commands = root / "commands"
        commands.mkdir()
        # Hosted runners have Rust preinstalled. Expose only installer tools to
        # both children, and omit the runner's credentials and language homes.
        for name in ("sh", "curl", "tar", "gzip", "uname", "mktemp", "mkdir", "install",
                     "mv", "rm", "shasum", "sha256sum"):
            executable = shutil.which(name)
            if executable:
                (commands / name).symlink_to(executable)
        environment = {
            "PATH": str(commands),
            "BURR_INSTALL_DIR": str(root / "installed"),
            "BURR_VERSION": version,
            "TMPDIR": str(root),
            "BURR_CACHE_DIR": str(root / "cache"),
            "BURR_VIEWER_NO_OPEN": "1",
        }
        for name in ("cargo", "rustc", "rustup"):
            if shutil.which(name, path=environment["PATH"]):
                raise RuntimeError(f"Unexpected Rust command: {name}")
        print("Installing with an empty environment and no Rust on PATH", flush=True)
        subprocess.run(
            [shutil.which("bash"), "-c",
             "set -euo pipefail; curl -fsSL https://burr.sh/install.sh | sh"],
            env=environment, check=True, timeout=420,
        )
        environment["PATH"] = os.pathsep.join(
            (environment["BURR_INSTALL_DIR"], str(commands)))
        actual = subprocess.check_output(
            ["burr", "--version"], env=environment, text=True, timeout=10).strip()
        if actual != version:
            raise RuntimeError(f"Expected version {version}, got {actual!r}")
        print(f"Installed version matches burr-v{version}", flush=True)

        models = root / "models"
        models.mkdir()
        fixture = Path(__file__).resolve().parent.parent / "tests/fixtures/interference/separated.step"
        shutil.copyfile(fixture, models / fixture.name)
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            environment["BURR_VIEWER_PORT"] = str(reservation.getsockname()[1])
        base_url = f"http://127.0.0.1:{environment['BURR_VIEWER_PORT']}"
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        log_path = root / "burr.log"
        with log_path.open("w") as log:
            process = subprocess.Popen(
                ["burr", str(models)], env=environment, cwd=models,
                stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
            )
            try:
                deadline = time.monotonic() + 60
                while True:
                    if process.poll() is not None:
                        raise RuntimeError(f"Burr exited before health: {process.returncode}")
                    try:
                        with opener.open(base_url + "/api/health", timeout=2) as response:
                            if response.status == 200:
                                break
                    except (urllib.error.URLError, TimeoutError):
                        pass
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Timed out waiting for Burr health")
                    time.sleep(0.25)
                with opener.open(base_url + "/api/checks?path=separated.step", timeout=30) as response:
                    report = json.load(response)
                expected = {
                    "schema_version": "burr.checks.v1",
                    "check_id": "assembly-interference",
                    "outcome": "pass",
                    "component_count": 2,
                    "checked_pair_count": 1,
                    "pair_set_complete": True,
                    "findings": [],
                }
                for key, value in expected.items():
                    if report.get(key) != value:
                        raise RuntimeError(f"Unexpected check report: {json.dumps(report)}")
                if report.get("unresolved_pairs", []):
                    raise RuntimeError(f"Unexpected unresolved pairs: {json.dumps(report)}")
                print("separated.step: pass, one checked pair, no findings or unresolved pairs", flush=True)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
                log.flush()
                print(log_path.read_text(), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    smoke(parser.parse_args().version)
