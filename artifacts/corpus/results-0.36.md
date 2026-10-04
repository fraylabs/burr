# Burr 0.36 real-assembly integration

Measured all 21 original multipart STEP models and the six original repros; report updated 2026-10-04T13:27:52+00:00.

7 of 21 models now agree with the conclusive OCCT assembly verdict, compared with 4 before. Classifications: {'incomplete': 14, 'correct': 7}. Burr verdicts: {'incomplete': 14, 'fail': 6, 'pass': 1}.

Classification describes the overall assembly verdict, not pair-list equivalence. A Burr or OCCT incomplete/timeout result remains **incomplete**; agreement between two incomplete results does not establish correctness.

Integrated Burr code: `ecdb30dc7668de21b8e7d5ce5b9d1ff30d0b4b72` (based on merged #37); Look fork branch `burr`: `08735d52d38c3782fa9f2157256bf2aa9f9d1e51`. Baseline Burr: `29dff12da0669a2e82c7283bf8a4695c02069825` (0.35.0), Look: `dc6687c5a579cee5cdfe91411d30bb71d19a5ac6`.

## Results

| Assembly | MiB | Parts before → after / OCCT | Faces lost before → after | Load s before → after | Check s before → after | Peak MiB after | Burr before → after (pairs after) | OCCT (pairs) | Classification before → after |
|---|---:|---:|---:|---:|---:|---:|---|---|---|
| openamr-platform-hw / MMP.00.00.00.000 full assembly.STEP | 24.77 | — → 1 / 688 | ≥33 → 18 | crash 0.627 → 1.149 | not reached → 0.000031 | 518.750 | crash → incomplete (0) | fail (≥1; partial scan) | incomplete → incomplete |
| openamr-platform-hw / MMP.01.00.00.000 Cover assembly.STEP | 0.86 | 53 → 53 / 53 | 0 → 0 | 0.039 → 0.029 | 0.573 → 0.311 | 34.141 | fail → fail (17) | fail (22) | correct → correct |
| openamr-platform-hw / MMP.02.00.00.000 Base assembly.STEP | 1.14 | — → 43 / 43 | — → 4 | crash 0.112 → 0.057 | not reached → 0.000004 | 50.141 | crash → incomplete (0) | fail (42) | incomplete → incomplete |
| openamr-platform-hw / MMP.03.00.00.000 Wheel assembly.STEP | 7.84 | — → 97 / 97 | — → 14 | crash 0.849 → 0.359 | not reached → 0.000009 | 180.203 | crash → incomplete (0) | fail (62) | incomplete → incomplete |
| openamr-platform-hw / MMP.04.00.00.000 Center bracket assembly.STEP | 1.38 | — → 33 / 33 | — → 2 | crash 0.105 → 0.049 | not reached → 0.000005 | 64.141 | crash → incomplete (0) | fail (16) | incomplete → incomplete |
| Voron-0 / MiniSB Bowden.step | 12.87 | 1 → 31 / 31 | 480 → 0 | 2.113 → 0.873 | 7.034 → 0.321 | 345.266 | incomplete → incomplete (0) | timeout (—) | incomplete → incomplete |
| Voron-0 / Fly Gemini V2 Din Mount 2pc.step | 2.23 | 1 → 11 / 11 | 31 → 0 | 0.115 → 0.114 | 0.706 → 0.061 | 84.516 | incomplete → incomplete (0) | fail (8) | incomplete → incomplete |
| Voron-0 / Manta M4P Din Mount 2pc.step | 0.56 | 1 → 2 / 2 | 0 → 0 | 0.033 → 0.027 | 0.328 → 0.048 | 42.984 | incomplete → pass (0) | pass (0) | incomplete → correct |
| Voron-0 / MiniSB adxl mount adafruit 19mm c c.step | 7.78 | 1 → 11 / 11 | 98 → 1 | 2.310 → 1.008 | 31.235 → 0.000008 | 329.891 | incomplete → incomplete (0) | fail (3) | incomplete → incomplete |
| Voron-0 / V0.2R1 Master Assembly v63.step | 170.93 | 1 → 1691 / 1691 | 1045 → 8 | 25.479 → 12.077 | ~575.897 capped → 0.000094 | 2558.125 | timeout → incomplete (0) | fail (≥1; partial scan) | incomplete → incomplete |
| Voron-2 / Voron 2.4r2 Assembly.step | 230.33 | 1 → 1428 / 1428 | 11099 → 34 | 18.654 → 33.810 | ~582.653 capped → 0.000060 | 2717.953 | timeout → incomplete (0) | fail (≥1; partial scan) | incomplete → incomplete |
| Voron-Switchwire / Switchwire Assembly v1 STEP.step | 86.27 | 1 → 712 / 712 | 3347 → 51 | 7.463 → 5.996 | ~592.974 capped → 0.000014 | 2191.984 | timeout → incomplete (0) | fail (≥1; partial scan) | incomplete → incomplete |
| TradRack / tr-lower-carabiner-distribution-board.STEP | 5.03 | 1 → 6 / 6 | 2 → 2 | 0.290 → 0.119 | 7.609 → 0.000005 | 147.312 | incomplete → incomplete (0) | fail (5) | incomplete → incomplete |
| TradRack / tr-toolhead-board.STEP | 4.07 | 1 → 7 / 7 | 0 → 0 | 0.275 → 0.105 | 5.200 → 0.092 | 118.203 | incomplete → fail (6) | fail (6) | incomplete → correct |
| Faze4-Robotic-arm / Faze4 dist v2 STEP.step | 1.01 | 136 → 136 / 136 | 0 → 0 | 0.083 → 0.052 | 5.129 → 0.044 | 62.031 | fail → fail (9) | fail (9) | correct → correct |
| apollo-3d-enclosure / flat / assembly.step | 1.29 | 1 → 2 / 2 | 68 → 0 | 0.061 → 0.520 | 0.780 → 0.060 | 88.156 | incomplete → incomplete (0) | pass (0) | incomplete → incomplete |
| apollo-3d-enclosure / corner / assembly.step | 1.30 | 1 → 2 / 2 | 45 → 0 | 0.054 → 0.323 | 0.655 → 0.062 | 84.500 | incomplete → incomplete (0) | pass (0) | incomplete → incomplete |
| Adafruit CAD Parts / 1400 Push Button Power Switch.step | 3.59 | 1 → 15 / 15 | 0 → 0 | 0.270 → 0.214 | 6.860 → 0.326 | 202.047 | incomplete → fail (4) | fail (11) | incomplete → correct |
| Adafruit CAD Parts / 1185 Massive Arcade Button 100mm.step | 0.77 | 1 → 5 / 5 | 36 → 0 | 0.167 → 0.414 | 0.258 → 0.087 | 95.781 | incomplete → incomplete (0) | incomplete (0) | incomplete → incomplete |
| text-to-cad-assembly / pick place arm.step | 0.45 | 17 → 17 / 17 | 0 → 0 | 0.028 → 0.016 | 0.084 → 0.012 | 28.188 | fail → fail (3) | fail (3) | correct → correct |
| text-to-cad-assembly / automation control unit.step | 0.86 | 20 → 20 / 20 | 0 → 0 | 0.053 → 0.039 | 0.276 → 0.030 | 48.500 | fail → fail (12) | fail (12) | correct → correct |

## What improved and what remains

All 13 previously flattened inputs now match the OCCT component count; 20 of 21 counts match overall. The full robot still falls back to one mesh and explicitly reports the unresolved source graph. There are no crashes or timeouts in the integrated 21-model run. The nullable-dollar repro now retains both occurrences, and both reduced Apollo faces render again. The two-part contact repro changes from false interference to a correct pass.

Source-face loss falls from 16,251 to 96 on the inputs whose baseline import completed. Across all 21 integrated models there are 134 losses in nine models; the four previously crashing inputs did not have complete baseline loss counts, so 134 is not directly comparable with 16,251.

The remaining import diagnoses are concrete: all 26 surface-conversion refusals are source TOROIDAL_SURFACE records with negative major radii; 42 Switchwire faces fail wire closure; the other 66 losses are periodic-chart parity/constraint/degeneracy refusals (33 ContradictoryDualParity, 12 ConstraintInsertionIncomplete, 11 NoOddParityRegion, 9 RejectedDegenerate, 1 ConstraintRoleMissing). Negative-radius handling is now a refusal, not support for that surface convention. V0 retains its 1,691 occurrences but includes two definitions with no source geometry.

The full robot graph fails while following two source SHAPE_REPRESENTATION_RELATIONSHIP records to MANIFOLD_SURFACE_SHAPE_REPRESENTATION (#55392 and #288197). Those entity variants are absent from Truck’s shape-representation holder table; a read-only probe confirms 197 decoded shape representations (107 basic + 90 advanced BREP), omitting the two manifold-surface representations. Supporting that representation variant is the next structure fix; this integration reports the loss explicitly instead of declaring a single flattened body safe.

Several meshes remain open even though every declared face contributes triangles: Bowden, Fly, both Apollo enclosures, and the arcade button. A read-only probe of production mesh preparation finds 120 boundary edges and 100 nonmanifold edges in Apollo flat Solid #15; corner Solid #15 has 128 boundary edges and 100 nonmanifold edges. Their other solids are closed. Fly has nonmanifold edges in five of its six definitions, including a bracket with zero boundary edges but two nonmanifold edges. The best working guess is nonconforming boundary sampling or overlapping triangulation; simple near-vertex welding is not an adequate explanation for Apollo, whose nearest distinct boundary vertices are over 0.04 mm apart. See topology-0.36.json and the reproducible corpus-topology.rs probe. Zero lost faces does not prove a closed mesh. The arcade spring is also invalid in the OCCT reference. Bowden has no conclusive exact verdict because its recorded OCCT run timed out.

A correct overall fail still does not certify its pair list. The cover reports 17 pairs versus OCCT 22, and the Adafruit switch reports 4 versus 11; those lists need a placement-matched exact comparison. Faze4 and the two build123d examples retain the reference pair counts (9, 3, 12). No assembly-level false positive or false negative is observed among the seven conclusive results.

## Remaining failures

- **openamr-platform-hw__MMP.00.00.00.000_full_assembly.STEP** — assembly_structure_lost: STEP assembly structure could not be resolved: failed to build the STEP assembly graph: failed to reference the geometry `shape_representation`; step_faces_lost: 18 of 7223 STEP faces were lost during import
- **openamr-platform-hw__MMP.02.00.00.000_Base_assembly.STEP** — step_faces_lost: 4 of 422 STEP faces were lost during import
- **openamr-platform-hw__MMP.03.00.00.000_Wheel_assembly.STEP** — step_faces_lost: 14 of 2411 STEP faces were lost during import
- **openamr-platform-hw__MMP.04.00.00.000_Center_bracket_assembly.STEP** — step_faces_lost: 2 of 570 STEP faces were lost during import
- **Voron-0__MiniSB_Bowden.step** — open_component_mesh: Could not prove a clean result because these tessellated components do not form closed solids: Cowling_ECAS04 v6, Cowling_PC4_M6 v6, Cowling_PC4_M10 v6, M3x35 BHCS, M3x35 BHCS, M3x16 BHCS, M3x8 FHCS, M2x6 SHCS, M3x8 FHCS, M2x6 SHCS, M2x6 SHCS, M2x6 SHCS, M3 Heatset Insert, M3 Heatset Insert, M3 Heatset Insert, M3 Hexnut, M3 Hexnut, 3010 Blower Fan (1), 3010 Blower Fan (1), PC4-M6_Collet, M10 Coupler, Plunger, Collet Body, Seal Retainer, X_Carriage, MGN7-H (1).
- **Voron-0__Fly_Gemini_V2_Din_Mount_2pc.step** — open_component_mesh: Could not prove a clean result because these tessellated components do not form closed solids: PCB DIN Clip, M2x10 self tapping screw for plastic:11, M2x10 self tapping screw for plastic:13, M2x10 self tapping screw for plastic:13, M2x10 self tapping screw for plastic:11, M3x8 BHCS, M3x8 BHCS, M3x8 BHCS, M3x8 BHCS, Fly_Gemini_V2_Bracket.
- **Voron-0__MiniSB_adxl_mount_adafruit_19mm_c_c.step** — step_faces_lost: 1 of 3672 STEP faces were lost during import
- **Voron-0__V0.2R1_Master_Assembly_v63.step** — assembly_structure_lost: STEP leaf component 'Threaded Inserts (1) (1)' has no definition geometry; assembly_structure_lost: STEP leaf component 'Threaded Inserts (1)' has no definition geometry; step_faces_lost: 8 of 60543 STEP faces were lost during import
- **Voron-2__Voron_2.4r2_Assembly.step** — step_faces_lost: 34 of 100800 STEP faces were lost during import
- **Voron-Switchwire__Switchwire_Assembly_v1_STEP.step** — step_faces_lost: 51 of 35873 STEP faces were lost during import
- **TradRack__tr-lower-carabiner-distribution-board.STEP** — step_faces_lost: 2 of 1527 STEP faces were lost during import
- **apollo-3d-enclosure__flat__assembly.step** — open_component_mesh: Could not prove a clean result because these tessellated components do not form closed solids: Solid #15.
- **apollo-3d-enclosure__corner__assembly.step** — open_component_mesh: Could not prove a clean result because these tessellated components do not form closed solids: Solid #15.
- **Adafruit_CAD_Parts__1185_Massive_Arcade_Button_100mm.step** — open_component_mesh: Could not prove a clean result because these tessellated components do not form closed solids: Holder, Actuator, Spring, Dome.

## Repros

| Input | Parts before → after | Faces lost after | Load s before → after | Check s before → after | Burr before → after | Interpretation |
|---|---:|---:|---:|---:|---|---|
| 01-assembly-pds-dollar.step | 1 → 2 | 0 | 0.074 → 0.007 | 0.169 → 0.003 | incomplete → incomplete | open_component_mesh |
| 01-assembly-pds-empty-strings.step | 2 → 2 | 0 | 0.015 → 0.008 | 0.573 → 0.004 | incomplete → incomplete | open_component_mesh |
| 02-edge-conversion-face-1073.step | — → 1 | 0 | 0.003 → 0.042 | — → 0.000001 | load error → incomplete | Isolated source face, not a two-component clearance test. assembly_required |
| 02-edge-conversion-minimal-face.step | — → 1 | 0 | 0.018 → 0.000640 | — → 0.000002 | load error → incomplete | Isolated source face, not a two-component clearance test. assembly_required |
| 03-negative-torus-face.step | — → — | 1 | — → 0.000584 | — → — | crash → load error | Isolated source face, not a two-component clearance test. failed to load STEP scene '/Users/brianlim/coding/fray/repos/burr/artifacts/corpus/repros/03-negative-torus-face.step': STEP tessellation produced no triangles: 1 of 1 STEP faces were lost |
| 04-mesh-contact-pair.step | 2 → 2 | 0 | 0.066 → 0.019 | 0.371 → 0.011 | fail → pass | OCCT pass (0 pairs): correct |

## Measurement limits

- Fresh process per input, one model at a time, release profile, four Cargo jobs, shared `/tmp/burr-build.lock` and `/tmp/burr-target`. Source SHA-256 values are verified before measurement. Load covers read/parse/tessellation/scene compilation; check covers production `analyze_scene` and excludes the browser, API scene clone and server cache. OS filesystem caches are uncontrolled.
- The integrated harness records source-definition face statistics directly from Look, counting shared source shells once. The before figures use the baseline diagnostic warning counts; baseline crashes only give partial refusal counts. The isolated negative-torus face is expected to return a typed no-geometry load error rather than panic.
- 600 seconds and 9 GiB per process; caps and load errors remain incomplete. Peak RSS is OS `wait4` maximum for load plus check. This is the same shared Apple M1 Pro, 16 GiB, macOS 27.0 machine; timings are observations under uncontrolled system load, not isolated performance claims.
- Reused exact reference: CadQuery 2.8.0 / OCP 7.9.3.1.1, XCAF leaf occurrences with world placements, BREP Common for strict AABB candidates. Flat one-leaf multipart exports are split by solid. Positive common volume must exceed max(1e-6 mm³, smaller-part volume × 1e-9); touching is not interference. Large reference scans stop at one valid positive-volume witness, so their pair count is a lower bound. MiniSB Bowden reference timed out; the arcade-button spring is OCCT-invalid. Neither can be certified correct against that reference.
- Corpus models and raw repro geometry remain external measurement inputs under their original licences. [sources.csv](sources.csv) records URLs, licences, repository revisions and SHA-256 values. Burr receives no GPL/CC CAD fixture in this change. The nullable Look regression is synthetic; the two source-derived face snippets in Look have separate source notices/licences.

## Reproduce

```sh
# Put the original corpus at CORPUS, using sources.csv and the original reducers.
mkdir -p examples
cp artifacts/corpus/scripts/corpus-bench.rs examples/corpus-bench.rs
until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done
CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target cargo build --release --locked --bin burr --example corpus-bench
rmdir /tmp/burr-build.lock
until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done
CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target /tmp/burr-occt-venv/bin/python artifacts/corpus/scripts/run_integrated.py --corpus "$CORPUS" --manifest artifacts/corpus/baseline-0.35.json --output artifacts/corpus/logs-0.36 --binary /tmp/burr-target/release/examples/corpus-bench
rmdir /tmp/burr-build.lock
```

The runner uses the recorded baseline JSON as its 21-model manifest and appends all six `.step` repros. It skips completed runs; choose an empty output directory for a new measurement. Raw integrated reports, metrics and diagnostic JSONL files are retained under `logs-0.36/`. Aggregate before/after rows are in [results-0.36.json](results-0.36.json).
