# Combined STEP imports on Burr 0.38.3

The original 0.38.0 baseline is Burr `d41b884` with Look `d1583a7`. Current main is Burr `1c17a588f6aca8bbe7bac4f5c794c95383fd92cd` (0.38.3) with Look `45f1d77986a59aef48357c6aa0ad4c5edff28e72`. The combined measurement uses Burr `b485c47712372109e9b88cc37acf54f7edbcb600` and Look `burr-0.39` at `58c39f4ba9ebabd28832e5c55193f85d9cc5286d`. The measured source is rebased onto current main, including the binary viewer, viewer cache version v7 and #49’s evidence.

All 21 corpus models and six repros were rerun sequentially under the shared build lock with four Cargo jobs. The protected set retains 64 verified true pairs and zero false pairs. Bowden retains nine true pairs and zero false pairs. Center retains its eight original M8/bracket pairs. No wrong conclusive verdict was produced, and every previously confirmed pair from both baselines is retained.

The manifold-surface representation decoder restores all 688 full-robot occurrences. Incomplete imports and unresolved contacts retain explicit reasons.

The combined Look pin contains the cherry-picked manifold decoder at `033488f32abd36d14f70e999083ebdc62efde2e8` and signed-torus implementation at `539936c79e72a4732bc9c1efb3bdfd682971e710`, together with the 0.38 mesh policy and #49 closure axes. The original `5a838db` pin in the historical evidence is a separate branch lineage; restoring that pin would discard the combined mesh work.

## Combined pair and verdict table

Each arrow shows **0.38.0 → current main → combined**. Exact pair checks refer to the combined result.

| Model | Parts | Lost faces | Burr verdict | Confirmed | OCCT true / false | Unresolved |
|---|---:|---:|---|---:|---:|---:|
| openAMR full robot | 1 → 1 → 688 | 17 → 17 → 5 | incomplete → incomplete → incomplete | 0 → 0 → 0 | — (refused) | 0 |
| openAMR Cover | 53 → 53 → 53 | 0 → 0 → 0 | fail → fail → fail | 16 → 16 → 16 | 16 / 0 | 25 |
| openAMR Base | 43 → 43 → 43 | 4 → 4 → 0 | incomplete → incomplete → fail | 0 → 0 → 16 | 16 / 0 | 26 |
| openAMR Wheel | 97 → 97 → 97 | 13 → 13 → 5 | incomplete → incomplete → incomplete | 0 → 0 → 0 | 0 / 0 | 0 |
| openAMR Center bracket | 33 → 33 → 33 | 2 → 2 → 0 | incomplete → incomplete → fail | 0 → 0 → 8 | 8 / 0 | 8 |
| MiniSB Bowden | 31 → 31 → 31 | 0 → 0 → 0 | fail → fail → fail | 9 → 9 → 9 | 9 / 0 | 76 |
| Fly Gemini mount | 11 → 11 → 11 | 0 → 0 → 0 | fail → fail → fail | 7 → 7 → 7 | 7 / 0 | 7 |
| Manta M4P mount | 2 → 2 → 2 | 0 → 0 → 0 | pass → pass → pass | 0 → 0 → 0 | 0 / 0 | 0 |
| MiniSB ADXL mount | 11 → 11 → 11 | 0 → 0 → 0 | incomplete → incomplete → incomplete | 0 → 0 → 0 | 0 / 0 | 13 |
| Voron V0.2 | 1691 → 1691 → 1691 | 6 → 6 → 6 | incomplete → incomplete → incomplete | 0 → 0 → 0 | — (refused) | 0 |
| Voron 2.4r2 | 1428 → 1428 → 1428 | 4 → 4 → 4 | incomplete → incomplete → incomplete | 0 → 0 → 0 | — (refused) | 0 |
| Switchwire | — → 712 → 712 | — → 46 → 46 | incomplete (cap) → incomplete → incomplete | 0 → 0 → 0 | — (refused) | 0 |
| TradRack lower board | 6 → 6 → 6 | 2 → 2 → 2 | incomplete → incomplete → incomplete | 0 → 0 → 0 | — (unchanged) | 0 |
| TradRack toolhead board | 7 → 7 → 7 | 0 → 0 → 0 | fail → fail → fail | 6 → 6 → 6 | 6 / 0 | 0 |
| Faze4 arm | 136 → 136 → 136 | 0 → 0 → 0 | fail → fail → fail | 9 → 9 → 9 | 9 / 0 | 126 |
| Apollo flat | 2 → 2 → 2 | 0 → 0 → 0 | pass → pass → pass | 0 → 0 → 0 | 0 / 0 | 0 |
| Apollo corner | 2 → 2 → 2 | 0 → 0 → 0 | pass → pass → pass | 0 → 0 → 0 | 0 / 0 | 0 |
| Push-button power switch | 15 → 15 → 15 | 0 → 0 → 0 | fail → fail → fail | 11 → 11 → 11 | 11 / 0 | 13 |
| 100 mm arcade button | 5 → 5 → 5 | 0 → 0 → 0 | incomplete → incomplete → incomplete | 0 → 0 → 0 | 0 / 0 | 7 |
| Pick/place arm | 17 → 17 → 17 | 0 → 0 → 0 | fail → fail → fail | 3 → 3 → 3 | 3 / 0 | 5 |
| Automation control unit | 20 → 20 → 20 | 0 → 0 → 0 | fail → fail → fail | 12 → 12 → 12 | 12 / 0 | 2 |

Every result that changed from either baseline and every protected model were checked with `compare_pairs.py`. Full occurrence matching was used where possible. Reported-pair-only matching retains the same 0.1 mm box limit, 0.2 mm uniqueness margin, source identity check, and 0.1 mm surface-sample limit. A refused comparison certifies no unreported pairs and is accepted only for an incomplete result with no findings. Bowden uses a fresh OCCT Common reference for all 18 previously reported pairs plus any new confirmed pairs, including source and Common validity checks; full assembly pair completeness remains unknown.

Source hashes, binary hashes, comparison scope, matching errors, exact positives left unconfirmed, and remaining reasons appear in [summary.json](summary.json). Detailed CAD, meshes, diagnostics, comparisons, and executables remain local.

## Why the combined result changes

The manifold decoder recovers assembly structure. Signed-major tori retain the native STEP UV chart and declared face sense, with source-bound reversal and folded-sheet inverse/normal handling.

The closure-axis fix from #49 is preserved: face-definition metadata accounts for the U/V transposition of reversed spline faces, with a source-surface fallback. This keeps evaluator subdivision in the correct native chart and restores normal Switchwire import speed. #49 records one non-manifold edge on V2.4’s Meanwell LRS-200-24 Body2 face 1797059. Its four lost faces, verdict, and pair lists remain unchanged; that periodic-seam issue remains deferred.

The 0.38 curved-contact guard included a large planar bracket’s full diagonal as curved sampling uncertainty, hiding resolved overlaps against smaller curved components. Only sides with sampled curvature contribute nominal curved uncertainty. Measured normal/chord deviation, boundary ambiguity for flat or missing normal samples, and the unoriented-pair guard remain. Flat samples do not certify a planar carrier or boundary. An authored multiscale cylinder/plate regression fails with the old guard and passes with the correction; tangent contact produces no finding.

Refining all torus neighborhoods perturbed Fly’s spline-containing bracket mesh. Refining only the signed carrier left Base’s positive companion outline 0.262 mm from its exact bounds, exceeding the unchanged 0.1 mm matching limit. The final rule applies the existing angular floor throughout a newly admitted shell containing a source-signed torus only when every carrier is a plane, cylinder, cone, or torus. Positive companion tori share canonical refined circle boundaries and compatible grids on both native angular axes. Mixed sphere/spline shells and shells without signed tori retain 0.38 torus sampling. Base’s finer mesh moves candidates that the contact guard cannot prove to unresolved. Regression tests cover baseline subdivision, signed provenance, companion carriers, and mixed-surface exclusion. Numerical interference, sampling, OCCT Common, and matching settings were not relaxed. The mesh cache revision is 9.

## Import timings on current main versus PR

| Model | Main 0.38.3 import | Combined import |
|---|---:|---:|
| Voron V0.2 | 13.56 s | 13.45 s |
| Voron 2.4r2 | 39.63 s | 37.36 s |
| Switchwire | 8.05 s | 7.11 s |

These are individual scene-import measurements from sequential release runs on the same Mac. Switchwire completes on both revisions; the gate rejects a capped load on either post-axis revision. The prior 0.38.0 600-second and 0.38.1 30-second caps remain historical evidence of the pre-existing slowdown. They are not completed checks.

## Remaining uncertainty

- openAMR full robot: 5 of 7223 STEP faces were lost during import; face refusals: RejectedDegenerate=3, NoOddParityRegion=2; OCCT comparison refused: Geometry mismatch for occurrence 87: 0.7662080017210326; OCCT reference did not finish its pair scan.
- openAMR Cover: Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.; unresolved candidates: below_coordinate_resolution=6, below_tessellation_resolution=19; 6 exact-positive pairs remain unconfirmed.
- openAMR Base: Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.; unresolved candidates: below_tessellation_resolution=26; 26 exact-positive pairs remain unconfirmed.
- openAMR Wheel: 5 of 2411 STEP faces were lost during import; face refusals: NoOddParityRegion=2, RejectedDegenerate=3; 62 exact-positive pairs remain unconfirmed.
- openAMR Center bracket: Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.; unresolved candidates: below_tessellation_resolution=8; 8 exact-positive pairs remain unconfirmed.
- MiniSB Bowden: Could not prove a clean result because these tessellated components do not form closed solids: Cowling_ECAS04 v6, Cowling_PC4_M6 v6, Cowling_PC4_M10 v6, 3010 Blower Fan (1), 3010 Blower Fan (1), M10 Coupler.; Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.; unresolved candidates: open_component_mesh=65, below_tessellation_resolution=10, below_coordinate_resolution=1.
- Fly Gemini mount: Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.; unresolved candidates: below_coordinate_resolution=2, below_tessellation_resolution=5; 1 exact-positive pairs remain unconfirmed.
- MiniSB ADXL mount: Could not prove a clean result because these tessellated components do not form closed solids: MiniSB_adxl_mount_adafruit_19mm_c_c (1), Male Pin Header v1:9 (1).; Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; unresolved candidates: open_component_mesh=10, below_coordinate_resolution=3; 3 exact-positive pairs remain unconfirmed.
- Voron V0.2: STEP leaf component 'Threaded Inserts (1) (1)' has no definition geometry; STEP leaf component 'Threaded Inserts (1)' has no definition geometry; 6 of 60543 STEP faces were lost during import; face refusals: NoOddParityRegion=5, ContradictoryDualParity=1; OCCT comparison refused: OCCT source occurrence 598 ('(Unsaved)/Skirts/Feet/Mains Inlet Foot/Hardware (28)/Threaded Inserts (2)/M3 Threaded Insert (1)/Threaded Inserts (1)') has no usable bounding box: Bnd_Box is void; OCCT source occurrence 598 ('(Unsaved)/Skirts/Feet/Mains Inlet Foot/Hardware (28)/Threaded Inserts (2)/M3 Threaded Insert (1)/Threaded Inserts (1)') has no usable bounding box: Bnd_Box is void.
- Voron 2.4r2: 4 of 100800 STEP faces were lost during import; face refusals: RejectedDegenerate=1, NoOddParityRegion=1, ConstraintInsertionIncomplete=2; OCCT comparison refused: Ambiguous placement for occurrence 30: 1.292938617170019, 30.39555388816504; OCCT reference did not finish its pair scan.
- Switchwire: 46 of 35873 STEP faces were lost during import; face refusals: conversion:edge_conversion:WireNotClosed=42, NoOddParityRegion=1, ContradictoryDualParity=2, RejectedDegenerate=1; OCCT comparison refused: Ambiguous placement for occurrence 146: 0.43297790499616234, 27.836017871854406; OCCT reference did not finish its pair scan.
- TradRack lower board: 2 of 1527 STEP faces were lost during import; face refusals: ContradictoryDualParity=2.
- Faze4 arm: Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; unresolved candidates: below_coordinate_resolution=126.
- Push-button power switch: Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.; unresolved candidates: below_coordinate_resolution=12, below_tessellation_resolution=1.
- 100 mm arcade button: Could not prove a clean result because these tessellated components do not form closed solids: Holder, Spring, Dome.; Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; unresolved candidates: open_component_mesh=6, below_coordinate_resolution=1.
- Pick/place arm: Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; unresolved candidates: below_coordinate_resolution=5.
- Automation control unit: Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.; unresolved candidates: below_coordinate_resolution=2.

## Repros

| Repro | Burr: 0.38 → combined | Lost faces | Reason / exact proof |
|---|---|---:|---|
| 01-assembly-pds-dollar.step | pass → pass | 0 → 0 | complete OCCT occurrence/pair comparison |
| 02-edge-conversion-face-1073.step | incomplete → incomplete | 0 → 0 | The selected STEP file does not expose at least two component occurrences. |
| 01-assembly-pds-empty-strings.step | pass → pass | 0 → 0 | complete OCCT occurrence/pair comparison |
| 02-edge-conversion-minimal-face.step | incomplete → incomplete | 0 → 0 | The selected STEP file does not expose at least two component occurrences. |
| 04-mesh-contact-pair.step | pass → pass | 0 → 0 | complete OCCT occurrence/pair comparison |
| 03-negative-torus-face.step | load error → incomplete | — → 0 | The selected STEP file does not expose at least two component occurrences. |

## Validation and historical evidence

`npm run check` passes strict Clippy, all 79 Burr tests, and the viewer proof. The eight reference-scope and source-bound tests pass. Three additional CLI regressions pass: missing boundary-nearest evidence rejects analytic agreement, and both reducers preserve Latin-1 source headers. All 26 historical face rows already have zero missing boundary-nearest results; the stricter assertion preserves their recorded result. Focused Look results: {'geometry_torus': 2, 'normal_derivatives': 1, 'stepio_lib': 58, 'real_step_faces': 11, 'step_input': 59, 'meshalgo_lib': 221, 'import_resilience': 3, 'mesh_conformity': 3, 'look_lib': 189, 'assembly': 13, 'step': 9, 'step_mesh_conformity': 4, 'meshalgo_existing_ignored': 1, 'step_input_existing_exclusions': 3}.

The combination corrects stale tests already failing on the exact 0.38 Look head: analytic source carriers versus snapped shared endpoints, and a tiny cylinder whose retained vertices exceed the unchanged source surface bound. A redundant test qualification blocked release compilation under `deny(warnings)`. Source-carrier assertions and typed refusal are preserved; admission limits are unchanged.

The original [0.38 conforming-mesh evidence](../conforming-mesh/README.md), [manifold evidence](../import-evidence/README.md), and [signed-torus evidence](../import-evidence/negative-tori.md) are retained. The latter two describe historical 0.37-based measurements; this report records the combined gate.

To rerun, build `scripts/corpus-bench.rs` and `scripts/scene-dump.rs` as temporary examples for each exact Burr/Look pin. Copy executables before another worker overwrites the shared target. Run `scripts/run_integrated.py` for the 21-model/six-repro manifests with `--switchwire-timeout 30`, then `scripts/measure_combined.py` with `--before`, `--current-main`, `--after`, `--scene-binary`, `--corpus`, `--reference-logs`, `--output`, and `--require-switchwire-complete`. Use the corpus OCCT environment. Wrap each build, test, corpus run, and comparison in the shared lock with `CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target`.
