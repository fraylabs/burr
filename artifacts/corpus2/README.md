# Burr 0.39.0 held-out assembly measurement

This corpus preserves a public **0.39.0 release baseline** and adds a separate **0.40.0 release rerun**, installed with the releases' checksum-verifying one-line installers. No Burr or Look source was changed or built for the measurements.

`sources.csv` records publicly offered downloads, pinned revisions, STEP SHA-256 hashes, archive members, applicable licences, attribution and exporter strings from the STEP headers. There are 27 distinct source files, with no hash, URL or repository overlap with the original corpus. The initial 25 were selected before testing; Positron and the Niryo gripper were added for exporter, size and domain coverage before their tests. Models were selected by provenance and multipart structure, not by Burr outcome. Single-part candidates were excluded based on source structure. The OCCT census is the final eligibility check.

The selection covers robots, printer/tool assemblies, a populated KiCad/FreeCAD board, laptop enclosures and hardware, furniture, and a gearmotor. Exporters include SolidWorks, Creo, FreeCAD and Autodesk Translation Framework. **This is a convenience sample, not a random sample of all CAD.** Several files share the Jubilee, Framework, FreeCAD-library and OpenArm projects. No source was downloaded behind a login. CAD files and detailed local evidence are not committed.

**Reference reliability correction:** independent source checks prove three pulley screw/nut overlaps despite valid empty OCCT Common results. The previous six-pair false-positive headline is suspended: those six are Common-zero disagreements with completed bounded source rechecks, not certified false pairs. Final measured scope: all 27 pinned models have terminal measurement and strict comparison receipts in frozen 0.39.0 and 0.40.0, and the five largest assemblies have actual Chrome overview inspections. All 27 remain incomplete/unverified for accuracy. The capped Biped removed-pair audit retains 23 pending pairs. The largest source census is 1,314 parts; the requested 2,000+-part coverage and full exporter spread remain unmet. See the source cross-check and retained receipts in results.md.

See [results.md](results.md) for the qualified verdict disagreements, ranked limitations, model table and Chrome inspection. A model classified `timeout` reached a Burr-stage guard; a reference-only timeout remains an unresolved reference and does not mean Burr timed out. Any reference timeout, ambiguous occurrence matching or incomplete pair scan is explicitly unresolved; agreement on an overall `fail` does not certify every pair.

## Reproduce

The work folder is under `~/coding/fray/.fray/tmp/`; earlier temporary folders were retired with their small evidence preserved. The reused OCCT environment is now `~/coding/fray/.fray/burr/occt-venv`. Use one heavy run at a time; release work has priority. The scripts now acquire `~/coding/fray/.fray/burr/build.lock` around each model command and release it afterwards, including ordinary failures. After acquisition, each shared lock contains an owner record identifying the thread, acquisition time and command. Release removes the owned directory and record. They never run CAD measurements concurrently. The reproducible runner leaves a 20-second gap after each job to give other workers polling the shared lock an opportunity to acquire it.

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

`occt_reference.py` reuses the existing corpus reference's XCAF occurrence traversal, OCCT Common-volume criterion and tolerances. Files above 10 MB or 150 components use its existing positive-witness shortcut; that scan cannot certify pair recall. Common zero is a disputed negative reference on fasteners; use independent source classification and winding checks before certifying separation. `compare_release.py` calls the existing `artifacts/corpus/scripts/compare_pairs.py`: 0.1-mm bounds, 0.2-mm alternative-placement margin, identity agreement when present and 0.1-mm surface-sample distance. If a reference scan is partial, reported pairs are checked separately, after the same strict occurrence validation, with valid source solids and valid OCCT Common. Subset comparisons certify only those reported pairs.

## Verdict-disagreement repro

The gearmotor reduction preserves original STEP entities and placement chains rather than changing the CAD with a new export:

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/FreeCAD-library__Yellow_gearmotor_L.step artifacts/corpus2/repros/01-gearmotor-contact.step --keep-name Metal --keep-name Yellow
until mkdir "$HOME/coding/fray/.fray/burr/build.lock" 2>/dev/null; do sleep 15; done
echo "${CORPUS_AGENT_ID:-corpus2} $(date) gearmotor release measurement" > "$HOME/coding/fray/.fray/burr/build.lock/owner"
"$OCCT_PYTHON" artifacts/corpus2/scripts/measure_release.py --binary "$CORPUS_WORK/bin0390/burr" --work-dir "$CORPUS_WORK" --model artifacts/corpus2/repros/01-gearmotor-contact.step --output artifacts/corpus2/repro-logs
rm -r "$HOME/coding/fray/.fray/burr/build.lock"
```

The source is attributed to its author in `sources.csv`, under CC BY 3.0. Retain that provenance when reproducing or sharing a derived CAD file. No CAD file is added by this PR.

## Local evidence summaries and Chrome

After the measurements and comparisons finish:

```sh
python3 artifacts/corpus2/scripts/summarize.py --root artifacts/corpus2
python3 artifacts/corpus2/scripts/report_table.py --root artifacts/corpus2
```

The table preserves reference scope, incomplete mapping and missing timings. A zero lost-face count with no declaration denominator means Burr logged no loss warning; it is not a claim that the model contains zero faces. The separate source_census.py script counts declared assembly-graph leaves for selection auditing, including wire/empty definitions; OCCT supplies the geometry census.

Outside a managed sandbox, render_model.py accepts the installed release binary, model path, evidence folder and Playwright module path; wrap each capture with the shared CAD lock. In the managed seat, direct Chrome launch is unavailable. The completed captures used Turnless browser.open for the Burr product and Playwright CDP, opened only owned tabs, retained tab-target and server ownership receipts, and verified their cleanup while preserving the managed browser. Isometric, front, top and right screenshots plus browser/WebGL receipts remain local. The five largest assemblies' isometric and front images were actually inspected. Overview images do not certify tiny or hidden face completeness, source placements or pair truth.

## Released-version follow-up

0.40.0 adds the gearmotor and spike fixes plus contact proofs. Its records live under `rerun-0400/`, separate from the immutable `logs/` baseline. The completed rerun contains all 27 pinned models, strict interference comparisons and every reported `contact_pairs` entry. Refusals and the Positron scene-harness error remain explicit. Bounded source selections and raw contact calculations are reported separately. The exact installed binary SHA-256 and version are saved locally in `rerun-0400/binary.json`.

```sh
BURR_VERSION=0.40.0 BURR_INSTALL_DIR="$CORPUS_WORK/bin0400" sh -c 'curl -fsSL https://github.com/fraylabs/burr/releases/download/burr-v0.40.0/install.sh | sh'
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py release --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs --binary "$CORPUS_WORK/bin0400/burr" --work-dir "$CORPUS_WORK" --limit 27
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py compare --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs --work-dir "$CORPUS_WORK" --limit 27
"$OCCT_PYTHON" artifacts/corpus2/scripts/run.py contacts --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs --work-dir "$CORPUS_WORK" --limit 27
"$OCCT_PYTHON" artifacts/corpus2/scripts/summarize_versions.py --root artifacts/corpus2 --evidence artifacts/corpus2/rerun-0400/logs
```

Copy the existing reference JSON/metrics into the version evidence folder before comparisons; do not overwrite the baseline. A sandbox that blocks `mktemp -d` needs explicit installer staging paths under its allowed work folder. Our run used a temporary `mktemp` compatibility shim for these two installer staging calls; the official installer, download URLs, archive checksum verification and binary were unchanged. All child processes use the allowed work folder as `TMPDIR`.

`verify_contacts.py` selects every contact occurrence through the existing strict identity, placement and surface matcher. It requires valid source shapes and valid, completed OCCT Common for each reported contact-or-separated pair. **Exactly zero volume** is recorded explicitly; any nonzero result is preserved, even below the ordinary interference reference threshold. Mapping refusals, invalid shapes and resource caps leave the relevant pairs unverified. Partial contact results survive a 600-second/9-GiB cap.

`summarize_versions.py` reports baseline coordinate-limit models separately: removal of that reason, pair-set completeness and independent pair verification are distinct results. Import-time face loss can suppress checking entirely; that is not a contact-proof success.

## Bounded independent source rechecks

`recheck_source_pair.py` binds one reported pair to its source occurrences through the unchanged strict matcher. It then records source validity, Common, bounded source-point classification and an independent solid-angle winding calculation on fine source meshes. Method 5 adds inward-normal and adjacent-face bisector probes, with source classification verifying entry into the sampled solid. A robust overlap certificate requires a point classified inside both source solids, winding inside both meshes, and more than 1e-6 mm clearance from both source boundaries. Shallower shared-interior samples can support bounded tolerance contact at the checked witnesses. Boundary disagreements, matching refusals, invalid sources and hard timeouts remain disputed. These finite samples do not establish exhaustive separation or a global overlap-depth or volume bound.

`run_source_rechecks.py` accepts a JSON job list of `[model_filename, [Burr_index, Burr_index]]`, takes the shared lock for each capped pair, writes its ownership receipt, and leaves a 20-second gap. It stops on an overlap certificate for immediate review. `summarize_source_rechecks.py` generates the per-pair method table and preserves pending and disputed rows.

When an underlying-surface normal projection falls outside its trimmed face, the checker retries from an actual closest point on that face. It retains the same 1e-6 mm on-face check, source-classified inward entry, sampling steps and overlap clearance rule. Failed projections remain disputed. The candidate pulley pair 12/14 needed this fallback; its original disputed receipt is preserved alongside the capped follow-up.

Both drivers accept `--stop-file`. By default, `run.py` watches `STOP_REQUESTED` in its evidence folder and `run_source_rechecks.py` watches it in its output folder. Create that file to let the current bounded job finish and pause before another job or while waiting for a lock. `run.py` exits 75 and writes `.driver-paused.json`; the source driver records a paused state without claiming phase completion. Before resuming, reconcile the controller and its children, remove only the requested stop file, and rerun the same command. Existing terminal receipts are retained and skipped; intentional scientific rechecks need separate output folders.
