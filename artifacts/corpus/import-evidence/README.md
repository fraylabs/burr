# Manifold-surface assembly import

This records the first import stage. The subsequent signed-major torus fix and its measurements are in [negative-tori.md](negative-tori.md).

The full openAMR robot now retains all 688 placed components instead of falling back to one flattened body. The two source `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` records (#55392 and #288197) have the same attributes as `SHAPE_REPRESENTATION`. Their shell models were already supported; decoding the representations makes the existing source relationships resolve.

Look branch: `fix/manifold-surface-assembly`, commit `e5066137f6d21d370e5c80af3a8e4e434fc78841`, based on `burr` at `08735d52d38c3782fa9f2157256bf2aa9f9d1e51`. Burr starts at origin/main `4f7f51796c1c8bbe9a3cd9a91ace96b7d3ee1be8` (0.37.0).

| Measurement | Before | After | OCCT reference |
|---|---:|---:|---:|
| Full robot components | 1 | 688 | 688 |
| Full robot assembly errors | 1 | 0 | — |
| Full robot lost faces | 18 | 18 | — |
| Full robot verdict | incomplete | incomplete | fail; partial pair scan |
| Other 20 models' component counts, face losses and verdicts | baseline | unchanged | unchanged |
| Seven completed checks: confirmed true / false pairs | 52 / 0 | 52 / 0 | 63 positive pairs |
| OCCT-positive pairs still inconclusive in those seven checks | 11 | 11 | — |

All 21 corpus models and six existing repros were rerun sequentially with the shared build lock and four Cargo jobs. All seven completed pair checks were rematched against placed OCCT occurrences with `compare_pairs.py`. There are no confirmed extra pairs and no newly wrong conclusive verdicts. The full robot remains incomplete because 18 faces are still missing; its partial OCCT pair scan cannot certify a complete pair list.

The import-count baseline is the recorded 0.36 run, which uses the identical Look revision pinned by Burr 0.37. The pair baseline is the 0.37 comparison summary. These are reused recorded baselines, not a newly executed baseline run. `manifold-summary.json` contains every after count, source hash and the two measured binary hashes. Detailed bench, scene, diagnostics, metrics and comparison JSON stay local under `/tmp/burr-import/evidence/manifold/`; no CAD geometry is committed. Source attribution remains in `../sources.csv` (openAMRobot/openamr-platform-hw, CERN-OHL-P-2.0).

The synthetic Look regression failed at the missing holder entry before the fix. After the fix it checks shell source identity, both occurrence transforms, and both linked and directly used manifold-surface representations. The focused STEP input suite passes 59 tests, with the same three documented resource/expectation tests excluded by the existing scoped verification script.

Remaining import work: 26 negative-major-radius torus refusals, 42 Switchwire wire-closure refusals, and 66 chart/constraint/degeneracy refusals. They are not reclassified by this structure change. Mesh closure repair remains separate. Full local `npm run check` passed (strict Clippy, 64 tests and viewer proof) before the Burr branch was pushed.

To reproduce, copy `scripts/corpus-bench.rs` and `scripts/scene-dump.rs` into a temporary Burr checkout's `examples/`, build both examples with the shared lock, and copy the binaries to a stable local location before another worker can overwrite `/tmp/burr-target`. Use the documented CadQuery/OCCT Python environment and the original corpus's models, six repros and OCCT logs:

```sh
until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done
CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target python artifacts/corpus/scripts/measure_imports.py \
  --corpus CORPUS_DIRECTORY --reference-logs OCCT_LOGS_DIRECTORY \
  --binary LOCAL_CORPUS_BENCH --scene-binary LOCAL_SCENE_DUMP --output LOCAL_EVIDENCE_DIRECTORY
measurement_status=$?
rmdir /tmp/burr-build.lock
exit "$measurement_status"
```

The harness verifies source hashes, refuses incomplete OCCT scans for pair comparisons, and stops on a new false pair, lost confirmed pair or wrong conclusive verdict. Its compact summary is intended for version control; its detailed outputs are local evidence.

The [Switchwire source-bound investigation](switchwire-loops.md) records why
all 42 disconnected/open source loops remain refused, with a reproducible OCCT
inspection script and a compact entity/error summary.
