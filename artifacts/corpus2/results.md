# Held-out Burr 0.39.0 results

**Confirmed wrong repro verdict: Burr 0.39.0 reports `fail` for two contacting gearmotor components; complete valid OCCT reports `pass`.** The original public six-part model has one false pair and four true pairs.

**On new models, 0 of 24 are fully verified correct so far**, excluding the separately confirmed false-positive gearmotor. Most have incomplete pair checks because of open meshes and coordinate limits; pending comparisons and reference limits also prevent certification. This is a verification gap, not evidence that all 24 verdicts are wrong.

**Provisional checkpoint: 25 of 27 selected models measured; 0 fully verified correct, 24 incomplete/unverified, 1 false positive.** No confirmed false negative, Burr crash or Burr timeout has been found in these 25. This is an interim result: eight pair comparisons, two source measurements and four Chrome inspections remain pending. The Positron reference was stopped for release priority and its partial output is excluded; it must be rerun. Derived repros are excluded from the denominator. No final 27-model accuracy claim is made.

## Confirmed false pair and wrong reduced verdict

Burr 0.39.0 reports interference between **Metal and Yellow** in the public FreeCAD-library yellow gearmotor. Independent OCCT checks find **zero common volume** between those two valid source solids. A 134,040-byte, two-component reduction preserving the original STEP geometry, entity IDs and ancestor placements changes the whole-model verdict: **Burr `fail`; OCCT `pass`**. The derived reduction is a repro, not another held-out assembly in the denominator.

On the original six-component assembly, strict occurrence identity, placement and source surface matching all succeed, with errors below 0.000003 mm. Burr reports five pairs: four match complete OCCT positives, and one is false. There are no missing positive pairs in that complete comparison. The original model's `fail` verdict is supported by the four real overlaps.

The false original pair is zero-based OCCT occurrences 0 (`Yellow gearmotor L shape/Motor/Metal`) and 3 (`Yellow gearmotor L shape/Box/Yellow`). Both source solids pass `BRepCheck_Analyzer`; OCCT Common is done and valid, with zero solids, zero faces and volume exactly 0 mm³. The minimum separation is 4.5999648534689186e-11 mm, consistent with contact. The reference positive-volume threshold for this pair is 5.376561799224365e-6 mm³. Strict comparison of the reduced assembly also finds exactly one extra Burr pair and no missing reference positives.

The original model is [Yellow_gearmotor_L.step, pinned public download](https://raw.githubusercontent.com/FreeCAD/FreeCAD-library/544a254e090eaf7bfbb6a9b69e249dc0d3d29d67/Electronics%20Parts/Motors/DC%20motor/Yellow_gearmotor/L_shape/Yellow_gearmotor_L.step), by hasecilu under CC BY 3.0. Its SHA-256 is `fe7343ace714992975bc9c34d6944295a9d80f0cf627771c5ce108a13d54360c`.

| Original zero-based pair | Burr 0.39.0 | Complete OCCT Common volume (mm³) | Comparison |
|---|---|---:|---|
| [0,2] Metal–Axis | interference | 62.83185307179585 | true pair |
| [0,3] Metal–Yellow | interference | **0 exactly** | **false pair** |
| [1,2] Plastic–Axis | interference | 21.991148575128552 | true pair |
| [1,5] Plastic–White | interference | 17.499999999999954 | true pair |
| [2,3] Axis–Yellow | interference | 15.707963267948962 | true pair |

Burr also leaves Metal–Plastic [0,1] unresolved under `below_coordinate_resolution`; its original pair set is incomplete. OCCT's complete scan reports no positive for that pair. This does not invalidate the four matched positives or the independently proved extra pair.

The successful reduction is `artifacts/corpus2/repros/01-gearmotor-contact.step`, **134,040 bytes, two components**, SHA-256 `bfaa2748bba69f73d7bf222eff82fb797277a649979fbaf57bbf7b52f1a77ca6`. It preserves source STEP IDs, geometry and ancestor placements without CAD re-export. Burr reports one Metal–Yellow pair [0,1], `fail`, and a complete pair set. OCCT reports `pass`, zero positives and a complete pair scan with both solids valid. Strict comparison finds one extra Burr pair and no missing positive. This is a **false fail on the reduction**, rather than a missed real overlap. The original assembly has a false pair; its overall `fail` remains supported by its other overlaps.

Reproduce from the pinned download in `sources.csv`:

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/FreeCAD-library__Yellow_gearmotor_L.step artifacts/corpus2/repros/01-gearmotor-contact.step --keep-name Metal --keep-name Yellow
/tmp/burr-occt-venv/bin/python artifacts/corpus2/scripts/measure_release.py --binary /tmp/burr-corpus2/bin/burr --model artifacts/corpus2/repros/01-gearmotor-contact.step --output artifacts/corpus2/repro-logs
/tmp/burr-occt-venv/bin/python artifacts/corpus2/scripts/measure_reference.py --model artifacts/corpus2/repros/01-gearmotor-contact.step --output artifacts/corpus2/repro-logs
/tmp/burr-occt-venv/bin/python artifacts/corpus2/scripts/compare_release.py --model artifacts/corpus2/repros/01-gearmotor-contact.step --evidence artifacts/corpus2/repro-logs --corpus-scripts artifacts/corpus/scripts
```

On the shared Mac, wrap **each** measurement or comparison command with the lock procedure in README.md. Source attribution and CC BY 3.0 terms are recorded in `sources.csv`. CAD repros and detailed logs remain local.

**Best root-cause guess:** near-coincident curved contact, float32 placement and faceting can create a long `surface_crossing` witness along the boundary without positive-volume penetration. The original reported segment is about 1.38 mm long, but its y displacement is only 6.02e-8 mm. Its endpoints are `[9.926700592041016,22.999999969888915,-0.6888622784459165]` and `[9.926700592041016,23.000000030111078,0.6888602677219704]`. The released [crossing path](https://github.com/fraylabs/burr/blob/b0bde98775903f61e9467cbccc5ee1135facabab/src/interference.rs#L705) checks interval length and a midpoint through `reliable_inside`; the [inside guard](https://github.com/fraylabs/burr/blob/b0bde98775903f61e9467cbccc5ee1135facabab/src/interference.rs#L628) includes coordinate and curved-surface proximity tests. Those guards still accept this exact-contact pair. The evidence does not establish whether placement rounding, faceting, surface classification or point-in-solid evaluation is the decisive cause. This is a geometric hypothesis, not a proof from instrumented checker internals.

## Released 0.40.0 rerun — in progress

The original 0.39.0 baseline above remains unchanged. A separately installed, checksum-verified 0.40.0 release is rerunning the original 25 models after the gearmotor fix, spike fix and contact proofs. Per-model interference comparison, exact-zero validation of every reported contact-or-separated pair, and unresolved reasons will be reported separately. No follow-up accuracy or contact-safety claim is made before those checks complete.

An early import regression is already measured: Jubilee's left double-pulley corner-bracket assembly loses source face 22181 (`ContradictoryDualParity`, source/synthetic `DuplicateTraversal`), returning `incomplete` with 1/510 faces lost and zero checked pairs. 0.39.0 imported it without face loss and reported a supported `fail` with unresolved pairs. The existing OCCT source reference validates all 23 components. Removal of its coordinate-limit reason is not a gain because the new import stops checking earlier. The exact cause of the new refusal is not established.

### Per-model 0.40.0 checkpoint

Release runs: 25/25 completed (8 `fail`, 16 `incomplete`, 1 `pass`). Verification columns remain provisional. “Complete” is Burr's pair-set claim, not yet certification of agreement; every reported contact remains subject to the independent exact-zero check. Empty contact sets require no CAD calculation. The table's zero counts never turn a pending comparison into agreement.

| Model | 0.39.0 coordinate limit | 0.40.0 verdict | Pair set complete | Interferences matched / extra / missing | Contact-or-separated checked zero / reported | Unresolved pairs and reasons |
|---|---|---|---|---|---|---|
| Framework-Laptop-13__Framework Laptop 13 CAD.stp | no | incomplete | False | refused | 0 / 0 | 0 (step_faces_lost) |
| Framework-Laptop-13__13_5_hinge_R_assy.stp | yes | incomplete | False | pending | 0 / 0 | 6 (open_component_mesh: 2, below_tessellation_resolution: 4) |
| Framework-Laptop-13__FWKNAQ9_G01_20210911.stp | no | incomplete | False | refused | 0 / 0 | 0 (step_faces_lost) |
| Framework-Laptop-13__FW_13_camera_module.stp | no | pass | True | refused | 0 / 0 | 0 (none) |
| Framework-Laptop-13__printable_case_full.stp | yes | fail | False | pending | 0 / 0 | 19 (below_tessellation_resolution: 16, open_component_mesh: 3) |
| ExpansionCards__ExpansionCard_SelfTapping.stp | no | incomplete | False | 0 / 0 / 2 | 0 / 0 | 3 (open_component_mesh: 3) |
| jubilee__jubilee.STEP | no | incomplete | False | pending | 0 / 0 | 0 (step_faces_lost) |
| jubilee__6x_well_plate_bed_assembly.STEP | no | incomplete | False | pending | 0 / 0 | 0 (step_faces_lost) |
| jubilee__left_double_pulley_corner_bracket_assembly.STEP | yes | incomplete | False | pending | 0 / 0 | 0 (step_faces_lost) |
| jubilee__tool_template_assembly.STEP | no | incomplete | False | pending | 0 / 0 | 21 (open_component_mesh: 21) |
| jubilee__passive_pen_tool_assembly.STEP | no | incomplete | False | pending | 0 / 0 | 73 (open_component_mesh: 73) |
| jubilee__bondtech_groovemount_extruder.STEP | yes | fail | False | pending | 0 / 0 | 138 (open_component_mesh: 120, below_tessellation_resolution: 13, below_coordinate_resolution: 5) |
| jubilee__ooze_wiper_assembly.STEP | no | fail | True | pending | 0 / 0 | 0 (none) |
| jubilee__camera_calibration_tool_jan_31_2020.STEP | no | incomplete | False | pending | 0 / 0 | 26 (open_component_mesh: 26) |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | yes | fail | False | pending | 0 / 0 | 631 (open_component_mesh: 451, below_tessellation_resolution: 164, below_coordinate_resolution: 16) |
| kicadStepUpMod__demo.step | no | incomplete | False | pending | 0 / 0 | 0 (step_faces_lost) |
| FreeCAD-library__Adirondack Chair.step | yes | fail | False | pending | 0 / 12 (unverified remainder) | 10 (below_coordinate_resolution: 10) |
| FreeCAD-library__wooden folding chair.step | yes | fail | False | pending | 0 / 0 | 22 (below_coordinate_resolution: 22) |
| FreeCAD-library__Wooden Folding Table.step | yes | incomplete | False | pending | 0 / 4 (unverified remainder) | 12 (below_coordinate_resolution: 12) |
| FreeCAD-library__Double glass doors with handles and transom.step | no | incomplete | False | pending | 0 / 0 | 0 (assembly_structure_lost) |
| FreeCAD-library__Yellow_gearmotor_L.step | yes | fail | False | pending | 0 / 0 | 2 (below_coordinate_resolution: 1, below_tessellation_resolution: 1) |
| FreeCAD-library__ComputerDesk (100 x 50 x 75 cm WDH).step | yes | fail | True | pending | 0 / 4 (unverified remainder) | 0 (none) |
| openarm_hardware__OpenArm_2.0.STEP | no | incomplete | False | pending | 0 / 0 | 0 (step_faces_lost) |
| openarm_hardware__OpenArmJIG.STEP | yes | incomplete | False | pending | 0 / 2 (unverified remainder) | 6 (below_coordinate_resolution: 2, open_component_mesh: 4) |
| openarm_hardware__OpenArm_Cell+OpenArm_2.0.STEP | no | incomplete | False | pending | 0 / 0 | 0 (step_faces_lost) |

### Where removed interference findings went

List audit of every old finding gives **49 removed findings: 48 unresolved, 0 contact-or-separated, 1 absent**. Component counts stayed unchanged, but the cross-version occurrence-index classification remains provisional until strict source mapping confirms each pair.

| Model | Removed old findings | Unresolved in 0.40.0 | Contact-or-separated | Absent | Reason |
|---|---:|---:|---:|---:|---|
| FreeCAD-library__Yellow_gearmotor_L.step | 1 | 1 | 0 | 0 | below_tessellation_resolution |
| jubilee__bondtech_groovemount_extruder.STEP | 9 | 9 | 0 | 0 | below_tessellation_resolution, open_component_mesh |
| jubilee__left_double_pulley_corner_bracket_assembly.STEP | 1 | 0 | 0 | 1 | step_faces_lost |
| jubilee__passive_pen_tool_assembly.STEP | 4 | 4 | 0 | 0 | open_component_mesh |
| jubilee__tool_template_assembly.STEP | 3 | 3 | 0 | 0 | open_component_mesh |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | 31 | 31 | 0 | 0 | below_tessellation_resolution |

The sole absent pair is pulley Burr [18,22], bracket against M4 shoulder screw. Its preserved 0.39.0 subset comparison strictly maps it to OCCT [0,1], with valid exact Common volume **0.13749159683157497 mm³** (threshold 1e-6 mm³), bounds/surface errors below 0.000008 mm. The 0.40.0 model is `incomplete` with zero pairs checked: loss of unrelated M5 screw face 22181 blocks the whole check. It is not called safe or given a pass verdict, but real positive evidence is absent from all pair lists. Original 23-part STEP is the confirmed repro. A source-preserving reduction retaining bracket, M4 screw and M5 screw placements (`12-pulley-positive-and-import-blocker.step`, 1,466,783 bytes) is prepared and **not yet validated**.

The other removed findings are explicitly unresolved, which is reduced usefulness rather than a conclusive wrong-pair claim. Their exact Common checks are the next priority. New tessellation-limit reasons are consistent with a more conservative trim-error budget, and the pen/template open-mesh reasons block previously reported evidence; diagnostics do not yet prove which change caused each transition. No removed old finding is provisionally classified as a #55 contact proof.

## Regressions vs 0.39.0

The same pinned source files were cold-loaded in the two installed releases. All 25 new release runs are finished, but the new strict comparisons and exact-zero contact validations are still in progress. A drop in raw pair count is not automatically a missed real overlap: the removed pair may have been false, or may now be unresolved. A confirmed false negative still requires a complete Burr pair set and strict exact-source agreement.

| Model | 0.39.0 → 0.40.0 | What is established | Pair-level state |
|---|---|---|---|
| Jubilee left double-pulley corner bracket | 0 → 1/510 faces lost; `fail` → `incomplete`; 1 → 0 reported pairs | source face 22181 now fails with `ContradictoryDualParity`; 23 exact source components valid | checking stops at import; retained-screw reduction 11 prepared, not yet validated |
| Full Jubilee | 2 → 3/9453 faces lost; both `incomplete` | old failed faces 286421/400279 no longer warn; new failed faces 202439/299236/389430, all parity failures | import remains incomplete; not one additional copy of the old same failed face |
| Jubilee passive pen | `fail` → `incomplete`; 4 → 0 reported pairs | old subset independently matched four true pairs; supported fail evidence is no longer reported | old full mapping was partial; new pair-level comparison pending |
| Jubilee tool template | `fail` → `incomplete`; 3 → 0 reported pairs | old subset independently matched three true pairs; supported fail evidence is no longer reported | old full mapping was partial; new pair-level comparison pending |
| Biped | 63 → 32 reported pairs; both `fail` | raw count reduction | old and new exact comparison pending; not yet proved lost true positives |
| Bondtech extruder | 18 → 9 reported pairs; both `fail` | raw count reduction | source reference partial/invalid; not yet proved lost true positives |

The gearmotor's 5 → 4 reported pairs is the expected removal of its confirmed false Metal–Yellow pair. The original model still reports the four prior true pairs, and leaves Metal–Yellow unresolved below tessellation resolution. Strict comparison and a fresh 0.40.0 run of the two-part repro remain pending. It is kept separate from the regression candidates.

The desk is the only one of the 11 former coordinate-limit models now reporting a complete pair set. Four no longer list the coordinate reason, but one is the pulley import failure; removing a reason by stopping checking earlier is not a gain. There are 22 contact-or-separated pairs reported across four models. Every one still requires an independently valid OCCT Common with exactly zero volume before it is described as verified.

## Classification and reference limits

The 25 completed measurement jobs yield raw Burr outcomes **11 `fail`, 13 `incomplete`, 1 `pass`**. Of the 17 comparison records, eight have full occurrence mapping and a full reference scan, five certify a subset, and four refuse comparison. Eight more comparisons are pending. These records match **57 reported positive pairs** to OCCT and contain **one confirmed extra pair**. This is a count within verified scopes, not a precision or recall estimate for unverified pairs.

Across the 25 references, 17 completed a full pair scan, six used the existing positive-witness shortcut, and two timed out. Five references contain invalid source components. The webcam's sole raw Burr `pass` remains unresolved against an invalid reference; it is not a confirmed false pass. Bed and cell reference timeouts are not Burr timeouts. Source counts are component occurrences, with compounds potentially containing several solids.

Use exclusive classes: correct, incomplete, false positive, false negative, crash, timeout. A confirmed extra pair takes precedence over an incomplete pair scan. A missing exact positive is a confirmed false negative only when Burr claims a complete pair set and strict full occurrence mapping succeeds. Partial OCCT scans, ambiguous source matching and unverified pairs remain incomplete; they are not silently counted as correct or false.

“Correct” means fully verified occurrence and pair agreement. A supported `fail` verdict can still be incomplete because Burr or the reference did not finish verifying every pair. Timings are cold-cache release viewer HTTP time (including import and viewer preparation), subsequent check time and OCCT import time; RSS includes the complete respective stage. See README.md for stage sampling and reference shortcut details.

## Which fixes could make incomplete models conclusive?

Rank fixes by the **24 incomplete/unverified models**, excluding the gearmotor false positive and both pending measurements. The single-fix candidate count below includes only models whose logged Burr reasons contain **that reason alone**. A fix must resolve every affected pair and avoid introducing wrong pairs. These are candidates for a conclusive Burr check, not promised fully verified successes: reference limits, occurrence matching and pending comparisons remain separate blockers. Multiple-reason models do not count as a single-fix win.

| Priority by single-fix candidates | Reason | Single-fix candidates / 24 | Candidates with full reference scan and strict full mapping already available | All affected / 24 | Minimal validated repro |
|---:|---|---:|---:|---:|---|
| 1 | `step_faces_lost` | 7 | 1 (PCB) | 7 | 04, board + LED; 2 parts, 291,185 bytes |
| 2= | `open_component_mesh` | 4 | 1 (expansion card) | 10 | 09, two jig occurrences; 2 parts, 90,297 bytes |
| 2= | `below_coordinate_resolution` | 4 | 2 (Adirondack chair, desk) | 10 | 10, Metal + Plastic; 2 parts, 82,830 bytes |
| 4 | `assembly_structure_lost` | 1 | 0; source scan complete, mapping refused | 1 | 05, door + wire; 2 leaves, 282,598 bytes |
| 5 | `below_tessellation_resolution` | 0 | 0 | 4 | 07, hinge axis + rivets; 3 parts, 109,112 bytes |

The seven face-loss candidates are full laptop, battery, Jubilee, bed, PCB, OpenArm 2.0 and OpenArm Cell. They do **not** share one proven meshing defect: the diagnostic mechanisms differ, two references timed out and some source compounds are invalid. Seven is the potential gain from resolving the whole class, not from fixing PCB parity alone.

The four open-mesh-only candidates are expansion card, tool template, passive pen and camera calibration tool. Three currently have incomplete occurrence mapping, so a closure fix alone cannot yet certify their pair accuracy. The four coordinate-only candidates are Adirondack chair, wooden folding chair, wooden folding table and desk; folding chair matching is refused and folding table mapping is partial.

Six other models need combined fixes: pulley bracket and OpenArmJIG have both coordinate and open-mesh reasons; hinge, printable case, Bondtech extruder and biped have coordinate, open-mesh and tessellation reasons. Fixing open meshes plus coordinate limits would remove the logged product blockers from 10 models (the eight single-reason candidates plus pulley bracket and jig); tessellation would still block the other four. The two remaining unverified models, webcam and ooze wiper, already have complete Burr pair sets and require reference/matching resolution rather than removal of a logged incomplete reason. All counts are provisional until reruns establish the new behavior.

Minimal repro generation commands and independent reference results for each class follow below. The compact jig and coordinate repros isolate their reason even where their original assemblies have more than one blocker.

## Ranked failure classes

The following frequencies are for the **25 completed source measurements**, counted once per model per reason. They overlap and are provisional until the two remaining models are measured. The false gearmotor pair is the highest-priority finding and appears first above, regardless of its frequency.

| Rank by affected models | Failure or refusal class | Models / 25 | Validated compact repro | Best explanation |
|---:|---|---:|---|---|
| 1 | `below_coordinate_resolution` | 11 | 10, Metal + Plastic; 2 parts, 82,830 bytes | finite mesh and transformed-placement error bands keep near-contact pairs unresolved |
| 2 | `open_component_mesh` | 10 | 09, two jig occurrences; 2 parts, 90,297 bytes | tessellation seam, weld or triangulation closure defect; the reduced source solids are valid |
| 3 | `step_faces_lost` | 7 | 04, board + LED; 2 parts, 291,185 bytes | several distinct conversion/meshing refusals; contextual scale may contribute to parity failures |
| 4 | `below_tessellation_resolution` | 4 | 07, hinge axis + reused rivets; 3 parts, 109,112 bytes | overlap evidence lies near curved boundaries below the nominal mesh probe scale |
| 5 | `assembly_structure_lost` | 1 | 05, door + wire annotation; 2 leaves, 282,598 bytes | unsupported wireframe representation breaks assembly traversal |

Positron's separately observed Chrome face loss is described below, but is not included in these 25-model counts before its measurement is finished.

Coordinate-resolution uncertainty reproduces on its own in an 82,830-byte Metal/Plastic reduction of the gearmotor. OCCT resolves two valid solids and 21 faces, completes the exact pair scan and returns `pass`. Burr imports both components without face loss, reports no interference, but leaves their only pair unresolved under `below_coordinate_resolution`, with a 0.000024182397079653934-mm coordinate-error bound. This is a conservative refusal rather than a false reported pair.

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/FreeCAD-library__Yellow_gearmotor_L.step artifacts/corpus2/repros/10-gearmotor-coordinate.step --keep-name Metal --keep-name Plastic
```

Coordinate-resolution uncertainty also reproduces in the 76,358-byte hinge base plus two rivet occurrences: three valid source solids, 63 faces and a complete OCCT `pass`. Burr reports one pair below coordinate resolution with an error bound of 0.00010409336683633261 mm, and another below the 0.0343993844492841-mm tessellation probe scale. Neither is counted as a false reported interference. The released checker combines mesh coordinate error with transformed placement uncertainty before accepting penetration; near-contact pairs can therefore remain unresolved even when the exact source pair has no reference-positive common volume. This explains the conservative refusal class, but does not establish why every affected model reaches it.

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/Framework-Laptop-13__13_5_hinge_R_assy.stp artifacts/corpus2/repros/08-hinge-base-rivets.step --keep-name GFW30_HINGE_R_BASE_BRK --keep-name GFW30_BASE_HINGE_RIVET_R
```

Open meshes reproduce in a 90,297-byte, two-occurrence reduction of OpenArmJIG. Both occurrences use the original `RACW60-60-10` definition and placements. OCCT resolves two valid solids and 72 faces, finds no candidate overlap, and completes with `pass`. Burr imports both components without face loss, but reports `open_component_mesh` for both and marks its pair set incomplete. This isolates tessellation closure from invalid source solids or lost faces. A seam, boundary weld or triangulation defect is the likely class of cause; this measurement does not establish the faulty source edge.

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/openarm_hardware__OpenArmJIG.STEP artifacts/corpus2/repros/09-jig-open-components.step --keep-name RACW60-60-10
```

Measure the reduction under the shared lock with the release and reference wrappers. The source attribution and CERN-OHL-S-2.0 licence are in sources.csv.

The populated KiCad/FreeCAD board loses source face 8490 during import: `ContradictoryDualParity`, arrangement stage, derived bucket `ParityContradiction`. The full 13-component source loses 1 of 401 faces. A 291,185-byte two-component source-preserving reduction (board plus LED) reproduces the same face and diagnostic, losing 1 of 59 faces. The independent reduced OCCT reference confirms two valid solids and 59 faces. It finds one real overlap of 0.002766632738945673 mm³; Burr marks the check incomplete at import rather than missing that overlap in a claimed complete scan. A 3,852-byte isolated-face attempt imported without loss, so it is not presented as a successful repro. The difference suggests import context or meshing scale matters; the precise cause is unproven.

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/kicadStepUpMod__demo.step artifacts/corpus2/repros/04-pcb-led-and-board.step --keep-name Pcb --keep-name led0603_
```

Run `measure_release.py` on the reduction under the shared lock and inspect its diagnostic JSONL. Keep the board's pinned source attribution and AGPL-3.0 provenance from sources.csv.

Face-loss diagnostics vary across models. The full laptop loses face 128324 with `BoundaryProjectionFailed` even though OCCT validates all 107 source solids. Jubilee loses two faces with `ContradictoryDualParity` and has 755 valid source components. OpenArm loses 12 faces (`EdgeTraversalUnresolved` and `NoOddParityRegion`); its reference also has two invalid source components, so those losses are not all evidence against valid geometry. The bed reports `ConstraintRoleMissing`. Battery loss includes `IntrinsicDegenerate` and other diagnostic forms, with invalid source compounds in the reference. These mechanisms must not be collapsed into one parity failure or described as all arising from valid source solids.

The early Positron Chrome import reports 183 lost faces: 167 `NoOddParityRegion`, 10 `ContradictoryDualParity` and six `EdgeTraversalUnresolved`. Its reference was interrupted for release priority and must be rerun before the final measurement. Diagnostic chord tolerance is 1.2955 mm on at least one sub-millimetre lost cylindrical face. A scene-scale meshing budget acting on small features is a plausible contributing factor; this observation does not prove the cause of every lost face.

The hinge's tessellation-scale contact limitation reproduces with three valid source solids (axis plus two reused rivet occurrences), 73 faces and a complete OCCT `pass`. Burr marks both axis/rivet pairs unresolved under `below_tessellation_resolution`; its probe resolution is 0.049572400652835205 mm. The 109,112-byte reduction preserves source geometry and placements. This is a conservative refusal, not a false reported interference.

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/Framework-Laptop-13__13_5_hinge_R_assy.stp artifacts/corpus2/repros/07-hinge-axis-rivets.step --keep-name GFW30_HINGE_R_AXIS_0703 --keep-name GFW30_BASE_HINGE_RIVET_R
```

The glass-door source also demonstrates `assembly_structure_lost`. It has one door compound containing 21 solids and two wireframe annotation leaves in OCCT; Burr falls back to one component and refuses a conclusive interference check. A 282,598-byte reduction keeping the door and one wire annotation reproduces the exact `failed to reference shape_representation` diagnostic. The wire annotation uses `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION`; an unsupported representation type in assembly traversal is the best explanation. The reduced OCCT reference resolves two valid leaf shapes, the door's 21 solids and 112 faces, and completes with `pass`. This count difference does not establish two missing solid parts.

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py 'artifacts/corpus2/models/FreeCAD-library__Double glass doors with handles and transom.step' artifacts/corpus2/repros/05-door-and-wireframe.step --keep-name Double_glass_doors_with_handles_and_transom_001 --keep-name Opening_indication
```

Run the released measurement wrapper on this reduction under the shared lock. The source is CC BY 3.0 with attribution in sources.csv. The wireframe-only reduction does not reproduce the assembly diagnostic: it is rejected for containing no shells or faces. The two-bracket hinge reduction also passes in both engines and is not a successful open-mesh repro.

## Model measurements

The first 25 rows with recorded timings have completed both measurement jobs. `pending` rows are not in the provisional 25-model denominator. `—` in the pair column means comparison pending, not zero pairs. Refused and subset comparisons do not certify recall. A zero loss count with no declaration denominator means no loss warning was logged. Burr viewer time includes import and viewer preparation; OCCT import includes occurrence expansion, source validity and bounding-box work. Peak RSS covers the respective process stage; all timings are seconds and memory is MiB.

| Model | Class | Parts Burr / OCCT | Lost / declared faces | Burr verdict | OCCT verdict | Strict pairs matched / extra / missing | Viewer s | Check s | Burr MiB | OCCT import s | OCCT total s | OCCT MiB |
|---|---|---:|---:|---|---|---|---:|---:|---:|---:|---:|---:|
| Framework-Laptop-13__Framework Laptop 13 CAD.stp | incomplete | 107 / 107 | 1 / 6652 | incomplete (partial) | fail (partial) | — | 7.088 | 0.059 | 636.031 | 48.380 | 66.139 | 755.297 |
| Framework-Laptop-13__13_5_hinge_R_assy.stp | incomplete | 6 / 6 | 0 / — | incomplete (partial) | pass | 0 / 0 / 0 | 0.123 | 0.408 | 44.641 | 2.487 | 3.004 | 461.828 |
| Framework-Laptop-13__FWKNAQ9_G01_20210911.stp | incomplete | 21 / 23 | 6 / 5752 | incomplete (partial) | fail (partial) | — | 0.664 | 0.060 | 480.453 | 7.095 | 11.107 | 641.453 |
| Framework-Laptop-13__FW_13_camera_module.stp | incomplete | 3 / 2 | 0 / — | pass | incomplete | refused | 0.124 | 0.111 | 109.312 | 3.264 | 7.108 | 545.516 |
| Framework-Laptop-13__printable_case_full.stp | incomplete | 15 / 15 | 0 / — | fail (partial) | fail | 3 / 0 / 2 | 0.310 | 4.422 | 236.328 | 3.408 | 7.488 | 498.781 |
| ExpansionCards__ExpansionCard_SelfTapping.stp | incomplete | 4 / 4 | 0 / — | incomplete (partial) | fail | 0 / 0 / 2 | 0.122 | 0.170 | 40.844 | 2.816 | 3.530 | 462.969 |
| jubilee__jubilee.STEP | incomplete | 755 / 755 | 2 / 9453 | incomplete (partial) | fail (partial) | — | 3.086 | 0.056 | 906.719 | 47.811 | 48.189 | 792.547 |
| jubilee__6x_well_plate_bed_assembly.STEP | incomplete | 98 / 98 | 1 / 1198 | incomplete (partial) | reference timeout | — | 1.707 | 0.055 | 170.578 | 9.062 | 600.090 | 746.672 |
| jubilee__left_double_pulley_corner_bracket_assembly.STEP | incomplete | 23 / 23 | 0 / — | fail (partial) | fail | 1 / 0 / 0 (partial mapping) | 1.395 | 0.859 | 211.562 | 3.715 | 63.163 | 553.266 |
| jubilee__tool_template_assembly.STEP | incomplete | 14 / 14 | 0 / — | fail (partial) | fail | 3 / 0 / 3 (partial mapping) | 0.185 | 0.564 | 77.844 | 4.280 | 65.181 | 690.203 |
| jubilee__passive_pen_tool_assembly.STEP | incomplete | 39 / 39 | 0 / — | fail (partial) | fail | 4 / 0 / 10 (partial mapping) | 0.451 | 2.643 | 158.109 | 4.530 | 225.975 | 727.828 |
| jubilee__bondtech_groovemount_extruder.STEP | incomplete | 60 / 60 | 0 / — | fail (partial) | fail (partial) | — | 1.946 | 131.933 | 533.234 | 13.175 | 22.987 | 707.531 |
| jubilee__ooze_wiper_assembly.STEP | incomplete | 6 / 6 | 0 / — | fail | fail | refused | 0.067 | 0.234 | 32.969 | 3.474 | 4.504 | 465.297 |
| jubilee__camera_calibration_tool_jan_31_2020.STEP | incomplete | 15 / 15 | 0 / — | incomplete (partial) | fail | 0 / 0 / 2 (partial mapping) | 0.331 | 0.348 | 115.641 | 6.923 | 120.910 | 604.250 |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | incomplete | 345 / 345 | 0 / — | fail (partial) | fail (partial) | — | 1.682 | 36.356 | 777.547 | 48.774 | 50.090 | 689.438 |
| kicadStepUpMod__demo.step | incomplete | 13 / 13 | 1 / 401 | incomplete (partial) | fail | 0 / 0 / 2 | 0.161 | 0.060 | 58.766 | 3.467 | 4.152 | 475.875 |
| FreeCAD-library__Adirondack Chair.step | incomplete | 36 / 36 | 0 / — | fail (partial) | fail | 38 / 0 / 4 | 0.070 | 0.171 | 25.141 | 2.426 | 3.343 | 456.844 |
| FreeCAD-library__wooden folding chair.step | incomplete | 20 / 20 | 0 / — | fail (partial) | fail | refused | 0.063 | 0.116 | 25.953 | 2.303 | 2.684 | 454.219 |
| FreeCAD-library__Wooden Folding Table.step | incomplete | 14 / 14 | 0 / — | incomplete (partial) | fail | 0 / 0 / 4 (partial mapping) | 0.062 | 0.056 | 22.203 | 2.254 | 2.564 | 453.734 |
| FreeCAD-library__Double glass doors with handles and transom.step | incomplete | 1 / 3 | 0 / — | incomplete (partial) | pass | refused | 0.066 | 0.057 | 22.781 | 2.215 | 2.476 | 451.844 |
| FreeCAD-library__Yellow_gearmotor_L.step | false positive | 6 / 6 | 0 / — | fail (partial) | fail | 4 / 1 / 0 | 0.068 | 0.282 | 30.484 | 2.230 | 2.683 | 458.000 |
| FreeCAD-library__ComputerDesk (100 x 50 x 75 cm WDH).step | incomplete | 9 / 9 | 0 / — | fail (partial) | fail | 4 / 0 / 0 | 0.066 | 0.060 | 20.734 | 5.729 | 6.049 | 454.719 |
| openarm_hardware__OpenArm_2.0.STEP | incomplete | 564 / 564 | 12 / 7520 | incomplete (partial) | fail (partial) | — | 2.349 | 0.060 | 869.094 | 31.983 | 32.434 | 804.938 |
| openarm_hardware__OpenArmJIG.STEP | incomplete | 8 / 8 | 0 / — | incomplete (partial) | pass | 0 / 0 / 0 | 0.125 | 0.164 | 69.656 | 2.263 | 2.583 | 459.719 |
| openarm_hardware__OpenArm_Cell+OpenArm_2.0.STEP | incomplete | 538 / 538 | 2 / 69931 | incomplete (partial) | reference timeout | — | 10.513 | 0.052 | 2902.953 | 64.962 | 600.113 | 2181.188 |
| Positron__PositronV3.2.2_2026-01-26.step | pending | — / — | — / — | — | — | — | — | — | — | — | — | — |
| ned2__ADAPTATIVE_GRIPPER_NED2_STEP.STEP | pending | — / — | — / — | — | — | — | — | — | — | — | — | — |

| Pending source | Measurement state | Comparison state |
|---|---|---|
| Positron V3.2.2 | reference interrupted by owner for release priority; archived and excluded; reference rerun and release timing required | pending |
| Niryo Ned2 gripper | reference and release jobs not started | pending |

The eight completed-model comparisons still pending are full laptop, battery, Jubilee, six-well bed, Bondtech extruder, biped, OpenArm 2.0 and OpenArm Cell.

| Comparison refusal | Recorded limit |
|---|---|
| Webcam | no Burr positives to check against a partial/invalid reference; overall pass unresolved |
| Ooze wiper | occurrence 2 bounds error 0.161996 mm exceeds the existing strict 0.1-mm bound limit |
| Wooden folding chair | occurrence 17 bounds error 0.193800 mm exceeds the same limit |
| Glass door | Burr 1 component versus 3 OCCT leaves; source contains a 21-solid door compound and two wire annotations |

Matching tolerances were not relaxed to turn these refusals into agreement.

## Chrome inspection

The five largest downloaded STEP files are Positron, OpenArm Cell, OpenArm 2.0, Jubilee and the biped. The final expanded OCCT census will verify whether this is also the five largest set by component occurrences. Chrome capture is serialized under the shared CAD lock; the actual installed release serves `burr .`, and the driver uses installed Google Chrome in solid mode at 1600 × 1100.

Positron is captured and its isometric and front images have been inspected. No long spikes or large holes are apparent at overview scale. The importer reports 183 missing faces; these small losses cannot be reliably located or ruled out from overview screenshots. Detached arms and a display are visible, but their source placements have not been independently verified; no defect is inferred from those positions. Chrome reports HTTP 200, 627 mesh definitions, 1,243 occurrences, WebGL error 0 and no JavaScript errors. The declared graph's 1,236 leaves is not a geometry count and must not be used to claim seven extra or missing parts.

| Model | Downloaded STEP bytes | Chrome capture and actual inspection | Observed defects / limits |
|---|---:|---|---|
| Positron V3.2.2 | 398,104,862 | isometric and front inspected | no obvious long spike or large hole at overview scale; importer logs 183 lost faces, not all visually locatable |
| OpenArm Cell | 194,761,438 | **pending** | no visual claim |
| OpenArm 2.0 | 47,788,783 | **pending** | no visual claim |
| Jubilee | 45,857,044 | **pending** | no visual claim |
| Biped | 26,548,352 | **pending** | no visual claim |

All four remaining captures and inspections require release-priority clearance. The completed Positron capture is reusable; no success is inferred from pending screenshots.

## Sample limits

The 27 public downloads are a convenience sample selected by provenance, domain and multipart source structure before testing. They have no old-corpus hash, URL or repository overlap. Several share projects, so they are not 27 independent exporter implementations. Exporters represented are SolidWorks, Creo, FreeCAD and Autodesk Translation Framework; the full requested exporter spread has not been achieved. A declared STEP assembly-graph census finds at most 1,236 expanded leaf occurrences (Positron); the sample does not meet the requested 2,000+-component coverage. Declared leaves may include wire or empty definitions, so final geometry component counts still come from OCCT. No accuracy generalization to all public assemblies is justified.
