# Burr 0.39.0 held-out assembly measurement

This corpus preserves a public **0.39.0 release baseline** and adds a separate **0.40.0 release rerun**, installed with the releases' checksum-verifying one-line installers. No Burr or Look source was changed or built for the measurements.

`sources.csv` records publicly offered downloads, pinned revisions, STEP SHA-256 hashes, archive members, applicable licences, attribution and exporter strings from the STEP headers. There are 27 distinct source files, with no hash, URL or repository overlap with the original corpus. The initial 25 were selected before testing; Positron and the Niryo gripper were added for exporter, size and domain coverage before their tests. Models were selected by provenance and multipart structure, not by Burr outcome. Single-part candidates were excluded based on source structure. The OCCT census is the final eligibility check.

The selection covers robots, printer/tool assemblies, a populated KiCad/FreeCAD board, laptop enclosures and hardware, furniture, and a gearmotor. Exporters include SolidWorks, Creo, FreeCAD and Autodesk Translation Framework. **This is a convenience sample, not a random sample of all CAD.** Several files share the Jubilee, Framework, FreeCAD-library and OpenArm projects. No source was downloaded behind a login. CAD files and detailed local evidence are not committed.

Baseline checkpoint: **at least 6 confirmed false pairs across two held-out models, all now unresolved in 0.40.0**. Of the 25 measured models, 0 are fully verified correct, 23 remain incomplete/unverified and 2 have confirmed false pairs. The two remaining measurements and pending comparisons/renders are explicit.

See [results.md](results.md) for the measured headline, false pairs, ranked limitations, model table and Chrome inspection. A model classified `timeout` reached a Burr-stage guard; a reference-only timeout remains an unresolved reference and does not mean Burr timed out. Any reference timeout, ambiguous occurrence matching or incomplete pair scan is explicitly unresolved; agreement on an overall `fail` does not certify every pair.

## Reproduce

The original baseline used `/tmp/burr-corpus2`. After the sandbox changed, the work folder moved under `~/coding/fray/.fray/tmp/` and the original temporary folders were retired. The reused OCCT environment is now `~/coding/fray/.fray/burr/occt-venv`. Use one heavy run at a time; release work has priority. The scripts now acquire `~/coding/fray/.fray/burr/build.lock` around each model command and release it afterwards, including ordinary failures. They never run CAD measurements concurrently. The reproducible runner leaves a 20-second gap after each job to give other workers polling the shared lock an opportunity to acquire it.

```sh
CORPUS_WORK="$HOME/coding/fray/.fray/tmp/<your-thread-id>"
OCCT_PYTHON="$HOME/coding/fray/.fray/burr/occt-venv/bin/python"
export TMPDIR="$CORPUS_WORK"
mkdir -p "$CORPUS_WORK"
BURR_VERSION=0.39.0 BURR_INSTALL_DIR="$CORPUS_WORK/bin0390" sh -c 'curl -fsSL https://github.com/fraylabs/burr/releases/download/burr-v0.39.0/install.sh | sh'
"$OCCT_PYTHON" artifacts/corpus2/scripts/download_sources.py artifacts/corpus2/sources.csv --root artifacts/corpus2
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py measure --root artifacts/corpus2 --binary "$CORPUS_WORK/bin0390/burr" --work-dir "$CORPUS_WORK"
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py compare --root artifacts/corpus2
```

The existing environment contains CadQuery 2.8.0, cadquery-ocp 7.9.3.1.1, NumPy 2.4.6, SciPy 1.17.1 and psutil 7.2.2. Scripts use macOS `wait4` RSS bytes. The source files belong under `artifacts/corpus2/models/`; logs, mesh buffers, Chrome screenshots and minimal CAD repros remain local.

`measure_release.py` starts a fresh installed Burr server and an empty viewer cache for each model, requests the real `/viewer` and `/api/checks` endpoints, and extracts placement, bounds and source surface samples from its served binary meshes. The meshed geometry is the actual release's viewer output. Component names come from Burr's check references where available; unreferred occurrences have an empty left-hand name, and still require unique placement plus source geometry checks. It does not use a separately built importer or checker.

The viewer HTTP duration includes parsing, tessellation, material preparation and viewer generation. Sampled load-stage transitions supply an upper bound on import time where captured; very fast imports can finish between samples. OCCT import time includes STEP loading, occurrence expansion, source validity checks and the bounding-box census; it is not a parser-only timing. Server RSS covers import, checking and mesh serving. Check time is measured separately after import. Lost-face counts are Burr's declared STEP face entities, not expanded per-occurrence face totals; OCCT face totals include repeated occurrences and must not be subtracted from Burr's declaration count. Both Burr and OCCT stages have a 600-second limit and a 9-GiB memory cap. The reproducible comparison runner applies the same caps and records unresolved comparison refusals on exhaustion.

`occt_reference.py` reuses the existing corpus reference's XCAF occurrence traversal, OCCT Common-volume criterion and tolerances. Files above 10 MB or 150 components use its existing positive-witness shortcut; that scan cannot certify pair recall. `compare_release.py` calls the existing `artifacts/corpus/scripts/compare_pairs.py`: 0.1-mm bounds, 0.2-mm alternative-placement margin, identity agreement when present and 0.1-mm surface-sample distance. If a reference scan is partial, reported pairs are checked separately, after the same strict occurrence validation, with valid source solids and valid OCCT Common. Subset comparisons certify only those reported pairs.

## False-pair repro

The gearmotor reduction preserves original STEP entities and placement chains rather than changing the CAD with a new export:

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/FreeCAD-library__Yellow_gearmotor_L.step artifacts/corpus2/repros/01-gearmotor-contact.step --keep-name Metal --keep-name Yellow
until mkdir "$HOME/coding/fray/.fray/burr/build.lock" 2>/dev/null; do sleep 15; done
"$OCCT_PYTHON" artifacts/corpus2/scripts/measure_release.py --binary "$CORPUS_WORK/bin0390/burr" --work-dir "$CORPUS_WORK" --model artifacts/corpus2/repros/01-gearmotor-contact.step --output artifacts/corpus2/repro-logs
rmdir "$HOME/coding/fray/.fray/burr/build.lock"
```

The source is attributed to its author in `sources.csv`, under CC BY 3.0. Retain that provenance when reproducing or sharing a derived CAD file. No CAD file is added by this PR.

## Local evidence summaries and Chrome

After the measurements and comparisons finish:

```sh
python3 artifacts/corpus2/scripts/summarize.py --root artifacts/corpus2
python3 artifacts/corpus2/scripts/report_table.py --root artifacts/corpus2
```

The table preserves reference scope, incomplete mapping and missing timings. A zero lost-face count with no declaration denominator means Burr logged no loss warning; it is not a claim that the model contains zero faces. The separate source_census.py script counts declared assembly-graph leaves for selection auditing, including wire/empty definitions; OCCT supplies the geometry census.

For a real Chrome capture, invoke render_model.py with the installed release binary, a model path, a local evidence output folder and the installed Playwright module path. Wrap each capture with the shared CAD lock. The driver uses the installed Google Chrome application and saves isometric, front, top and right screenshots plus browser/WebGL errors. Inspect the actual images before making geometry claims; a successful HTTP load does not prove a complete render.

## Released-version follow-up

0.40.0 adds the gearmotor and spike fixes plus contact proofs. Its records live under `rerun-0400/`, separate from the immutable `logs/` baseline. The rerun first measures the original 25 models, then compares their interference pairs and checks every `contact_pairs` entry independently. The two remaining source measurements and original Chrome inspections follow. The exact installed binary SHA-256 and version are saved locally in `rerun-0400/binary.json`.

```sh
BURR_VERSION=0.40.0 BURR_INSTALL_DIR="$CORPUS_WORK/bin0400" sh -c 'curl -fsSL https://github.com/fraylabs/burr/releases/download/burr-v0.40.0/install.sh | sh'
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py release --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs --binary "$CORPUS_WORK/bin0400/burr" --work-dir "$CORPUS_WORK" --limit 25
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py compare --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs --work-dir "$CORPUS_WORK" --limit 25
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py contacts --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs --work-dir "$CORPUS_WORK" --limit 25
"$OCCT_PYTHON" artifacts/corpus2/scripts/summarize_versions.py --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs
```

Copy the existing reference JSON/metrics into the version evidence folder before comparisons; do not overwrite the baseline. A sandbox that blocks `mktemp -d` needs explicit installer staging paths under its allowed work folder. Our run used a temporary `mktemp` compatibility shim for these two installer staging calls; the official installer, download URLs, archive checksum verification and binary were unchanged. All child processes use the allowed work folder as `TMPDIR`.

`verify_contacts.py` selects every contact occurrence through the existing strict identity, placement and surface matcher. It requires valid source shapes and valid, completed OCCT Common for each reported contact-or-separated pair. **Exactly zero volume** is recorded explicitly; any nonzero result is preserved, even below the ordinary interference reference threshold. Mapping refusals, invalid shapes and resource caps leave the relevant pairs unverified. Partial contact results survive a 600-second/9-GiB cap.

`summarize_versions.py` reports the 11 baseline coordinate-limit models separately: removal of that reason, pair-set completeness and independent pair verification are distinct results. Import-time face loss can suppress checking entirely; that is not a contact-proof success.
