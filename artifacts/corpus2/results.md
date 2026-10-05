# Held-out Burr 0.39.0 results

**Reference correction: a valid, empty OCCT Common is not sufficient to prove non-overlap.** Independent checks on three pulley M5 screw/t-nut pairs demonstrate real overlap even though direct Common returns exactly zero and a valid empty shape. The six earlier alleged false pairs (gearmotor plus five Biped pairs) are therefore **Common-zero disagreements: three bounded negatives and three disputed source checks**, not a certified false-pair count. They all remain unresolved in released 0.40.0.

**Current checkpoint: 25 of 27 source models measured; 0 fully verified correct, 25 incomplete/unverified.** No false-pair count is certified from Common zero alone. Two measurements, eight baseline pair comparisons and four Chrome inspections remain pending. The interrupted Positron reference remains excluded. Derived repros are excluded from the denominator.

**Measured gearmotor repro verdict disagreement: Burr 0.39.0 `fail`, OCCT Common reference `pass`.** The 134,040-byte two-part source-preserving repro and strict occurrence matching remain reproducible evidence. The independent source recheck is disputed at boundary points, so a true Burr false fail remains unproved. The original assembly also has four positive Common pairs.

## Gearmotor: zero Common and opposite reduced verdict


Burr 0.39.0 reports interference between **Metal and Yellow** in the public FreeCAD-library yellow gearmotor. Independent OCCT checks find **zero common volume** between those two valid source solids. A 134,040-byte, two-component reduction preserving the original STEP geometry, entity IDs and ancestor placements changes the whole-model verdict: **Burr `fail`; OCCT `pass`**. The derived reduction is a repro, not another held-out assembly in the denominator.

On the original six-component assembly, strict occurrence identity, placement and source surface matching all succeed, with errors below 0.000003 mm. Burr reports five pairs: four match complete OCCT positives, and one has zero Common volume. There are no missing positive pairs in that complete comparison. The original model's `fail` verdict is supported by the four real overlaps.

The Common-zero original pair is zero-based OCCT occurrences 0 (`Yellow gearmotor L shape/Motor/Metal`) and 3 (`Yellow gearmotor L shape/Box/Yellow`). Both source solids pass `BRepCheck_Analyzer`; OCCT Common is done and valid, with zero solids, zero faces and volume exactly 0 mm³. The minimum separation is 4.5999648534689186e-11 mm, consistent with contact. The reference positive-volume threshold for this pair is 5.376561799224365e-6 mm³. Strict comparison of the reduced assembly also finds exactly one extra Burr pair and no missing reference positives.

The original model is [Yellow_gearmotor_L.step, pinned public download](https://raw.githubusercontent.com/FreeCAD/FreeCAD-library/544a254e090eaf7bfbb6a9b69e249dc0d3d29d67/Electronics%20Parts/Motors/DC%20motor/Yellow_gearmotor/L_shape/Yellow_gearmotor_L.step), by hasecilu under CC BY 3.0. Its SHA-256 is `fe7343ace714992975bc9c34d6944295a9d80f0cf627771c5ce108a13d54360c`.

| Original zero-based pair | Burr 0.39.0 | Complete OCCT Common volume (mm³) | Comparison |
|---|---|---:|---|
| [0,2] Metal–Axis | interference | 62.83185307179585 | true pair |
| [0,3] Metal–Yellow | interference | **0 exactly** | **Common-zero disagreement** |
| [1,2] Plastic–Axis | interference | 21.991148575128552 | true pair |
| [1,5] Plastic–White | interference | 17.499999999999954 | true pair |
| [2,3] Axis–Yellow | interference | 15.707963267948962 | true pair |

Burr also leaves Metal–Plastic [0,1] unresolved under `below_coordinate_resolution`; its original pair set is incomplete. OCCT's complete scan reports no positive for that pair. The four matched positive Common results remain recorded separately from the Common-zero disagreement awaiting independent source revalidation.

The successful reduction is `artifacts/corpus2/repros/01-gearmotor-contact.step`, **134,040 bytes, two components**, SHA-256 `bfaa2748bba69f73d7bf222eff82fb797277a649979fbaf57bbf7b52f1a77ca6`. It preserves source STEP IDs, geometry and ancestor placements without CAD re-export. Burr reports one Metal–Yellow pair [0,1], `fail`, and a complete pair set. OCCT reports `pass`, zero positives and a complete pair scan with both solids valid. Strict comparison finds one extra Burr pair and no missing positive. This is a **fail/pass disagreement on the reduction**. Its former false-fail interpretation is provisional until independent source revalidation. The original assembly has a Common-zero interference disagreement; its overall `fail` remains supported by its other overlaps.

Reproduce from the pinned download in `sources.csv`, using `CORPUS_WORK` and `OCCT_PYTHON` from README.md:

```sh
python3 artifacts/corpus2/scripts/reduce_pair.py artifacts/corpus2/models/FreeCAD-library__Yellow_gearmotor_L.step artifacts/corpus2/repros/01-gearmotor-contact.step --keep-name Metal --keep-name Yellow
"$OCCT_PYTHON" artifacts/corpus2/scripts/measure_release.py --binary "$CORPUS_WORK/bin0390/burr" --work-dir "$CORPUS_WORK" --model artifacts/corpus2/repros/01-gearmotor-contact.step --output artifacts/corpus2/repro-logs
"$OCCT_PYTHON" artifacts/corpus2/scripts/measure_reference.py --model artifacts/corpus2/repros/01-gearmotor-contact.step --output artifacts/corpus2/repro-logs
"$OCCT_PYTHON" artifacts/corpus2/scripts/compare_release.py --model artifacts/corpus2/repros/01-gearmotor-contact.step --evidence artifacts/corpus2/repro-logs --corpus-scripts artifacts/corpus/scripts
```

On the shared Mac, wrap **each** measurement or comparison command with the lock procedure in README.md. Source attribution and CC BY 3.0 terms are recorded in `sources.csv`. CAD repros and detailed logs remain local.

**Best root-cause guess:** near-coincident curved contact, float32 placement and faceting can create a long `surface_crossing` witness along the boundary without positive-volume penetration. The original reported segment is about 1.38 mm long, but its y displacement is only 6.02e-8 mm. Its endpoints are `[9.926700592041016,22.999999969888915,-0.6888622784459165]` and `[9.926700592041016,23.000000030111078,0.6888602677219704]`. The released [crossing path](https://github.com/fraylabs/burr/blob/b0bde98775903f61e9467cbccc5ee1135facabab/src/interference.rs#L705) checks interval length and a midpoint through `reliable_inside`; the [inside guard](https://github.com/fraylabs/burr/blob/b0bde98775903f61e9467cbccc5ee1135facabab/src/interference.rs#L628) includes coordinate and curved-surface proximity tests. Those guards accept this Common-zero pair; exact source contact versus overlap is now being independently revalidated. The evidence does not establish whether placement rounding, faceting, surface classification or point-in-solid evaluation is the decisive cause. This is a geometric hypothesis, not a proof from instrumented checker internals.

## Biped: five Common-zero interference disagreements

The [SolidWorks biped source](https://raw.githubusercontent.com/open-dynamic-robot-initiative/open_robot_actuator_hardware/66af1522b4fba0ec4a1d7790e66f5e4652208d30/mechanics/biped_6dof_v1/cad_files/STEP/biped_6dof_v1.STEP), by Open Dynamic Robot Initiative under BSD-3-Clause, has five strictly mapped Common-zero interference disagreements in Burr 0.39.0. Its 345 source components are valid in the recorded reference census. These selected pairs pass strict old/new occurrence identity, placement and surface matching; both selected source solids and completed OCCT Common are valid.

| Burr 0.39.0 pair | Components | OCCT source pair | Common volume mm³ | Burr 0.40.0 |
|---|---|---|---:|---|
| [67, 73] | encoder_codewheel_pwb_mount / encoder_codewheel_5000cpr_pwb_7mm | [116, 117] | **0 exactly** | unresolved below tessellation resolution |
| [67, 74] | encoder_codewheel_pwb_mount / transmission_pulley_at3_t10_motor | [115, 116] | **0 exactly** | unresolved below tessellation resolution |
| [74, 87] | transmission_pulley_at3_t10_motor / transmission_timing_belt_at3_150_4 | [115, 138] | **0 exactly** | unresolved below tessellation resolution |
| [77, 85] | transmission_pulley_at3_t10_center / transmission_timing_belt_at3_201_6 | [127, 139] | **0 exactly** | unresolved below tessellation resolution |
| [118, 121] | transmission_pulley_at3_t30_output / transmission_timing_belt_at3_201_6 | [72, 91] | **0 exactly** | unresolved below tessellation resolution |

The largest bounds error across these selected pairs is 0.066930 mm, below the unchanged 0.1-mm matching limit; surface errors are below 0.000009 mm. The codewheel pair's reported tessellation probe is 0.0540497 mm. These are baseline interference/Common disagreements removed conservatively into unresolved status by 0.40.0. Independent non-overlap proof is pending. The Biped audit reached its 600-second reference guard after five checked pairs and one strict matching refusal; 25 removed Biped pairs remain pending. This is an audit timeout, not a Burr timeout. The full biped remains the confirmed disagreement repro. A source-preserving encoder mount/codewheel reduction, 13-biped-encoder-contact.step (350,080 bytes), is prepared and **not yet validated**. No wrong reduced verdict is claimed for it.

A long near-tangential surface-crossing witness and the new trim-error/sampling refusal are consistent with the curved-contact failure class; the precise checker mechanism is not instrumented. The gearmotor remains first because its two-part whole-model verdict disagreement against the Common reference is reproduced. Its ground-truth interpretation is now provisional.

## Six historical Common-zero source rechecks

All six historical disagreements completed strict source matching, valid Common, source seed-cube classification and independent source-mesh winding under per-pair 150-second caps. The original Burr witness points and surface-crossing endpoints were converted back into STEP coordinates and included as extra seeds. No sampled point was classified IN both source solids; no shared-interior certificate was found. No timeout or matching refusal occurred.

Biped [74,87], [77,85] and [118,121] have bounded negative evidence. The gearmotor and Biped [67,73]/[67,74] remain disputed: winding gives shared-inside values at source ON/ON points, or at an IN/OUT point on [67,73]. Numerical boundary disagreement is not a shared-interior certificate. None of these finite searches establishes exhaustive non-overlap, so the earlier definitive false-pair count remains suspended.

The method and full receipts are local under `baseline-disagreement-crosschecks/`.

Bounded source rechecks: 3 contact_or_separated_bounded, 3 disputed.

Each pair uses strict occurrence matching, valid source solids, OCCT Common, source point classification in bounded seed cubes, and independent solid-angle winding of fine source meshes. An overlap certificate also requires more than 0.000001 mm clearance to both source boundaries. Negative sampling is bounded; it does not establish exhaustive separation. Boundary disagreements, invalid sources, mapping refusals and timeouts remain disputed.

| Model | Burr pair | Status | Common mm³ | Source IN/IN samples | Winding IN/IN samples | Interior certificates | Limitation |
|---|---|---|---:|---:|---:|---:|---|
| FreeCAD-library__Yellow_gearmotor_L.step | 0/3 | disputed | 0.0 | 0 | 4 | 0 |  |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | 67/73 | disputed | 0.0 | 0 | 15 | 0 |  |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | 67/74 | disputed | 0.0 | 0 | 6 | 0 |  |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | 74/87 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | 77/85 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |
| open_robot_actuator_hardware__biped_6dof_v1.STEP | 118/121 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |

## Independent source cross-check: Common failure on pulley fasteners

The mesh worker's fallback candidate reports Burr [8,17], [9,19], [13,20], strictly mapped to OCCT [17,20], [18,21], [19,22]. These are candidate checks, not the released 0.40.0 dataset. **All three pairs overlap. Direct OCCT Common is wrong on all three.** Fresh source import validates each solid. Exact source classification at tolerance 1e-9 mm places all 27 points in a 0.004-mm cube inside both solids. A separate solid-angle winding algorithm on source tessellations (0.0005-mm tolerance; nut 90,480 triangles, screw 3,192) gives winding number 1 in both at the seed and opposite cube corners; outside controls give approximately zero. The seeds have positive clearance to every face of both source solids, so these are interior neighborhoods, not boundary contact.

| Candidate Burr pair | OCCT pair | Seed clearances to both source boundaries (mm) | Independent verdict | Nut Cut loss (mm³) | Screw Cut loss (mm³) |
|---|---|---|---|---:|---:|
| [8,17] | [17,20] | 0.2275990474 / 0.0098987827 | overlap | 0.1068060277 | 0.0038619979 |
| [9,19] | [18,21] | 0.2275973976 / 0.0098987827 | overlap | 0.1068060277 | 0.0038619979 |
| [13,20] | [19,22] | 0.0086465103 / 0.1483133669 | overlap | 0.2498401352 | **-1.1274076236** |

All direct Common results are done, valid and empty. On the first pair, sequential/non-destructive Common stays empty at fuzzy values 0, 1e-7, 1e-5 and 0.001 mm and after shape healing. Bilateral Cut produces inconsistent losses; it even increases the third screw volume. These Boolean volumes are not reliable total overlap measurements. Using interior radii equal to 80% of the minimum exact boundary distance gives contained-neighborhood volume lower bounds of about 2.0802e-6, 2.0802e-6 and 1.3864e-6 mm³. The overlap verdict rests on source interior classification and the independent winding check, not these failed Booleans.

The bounded check finished in 61.23 seconds under a 150-second hard timeout. The earlier broad grid was interrupted and its Boolean receipts retained. Reproduce with `scripts/crosscheck_pairs.py --model <pinned pulley STEP> --request <JSON list> --output <result.json> --corpus-scripts artifacts/corpus/scripts --timeout 150`, wrapped in the shared lock. Each request contains `burr_pair`, `occt_pair` and a source-coordinate `point`. The three seed points are `[24.096392393112183,29.34263029549296,75.40219670762737]`, `[24.096392393112183,29.342628388144327,47.902196707627375]` and `[47.46845161914826,29.48744597628384,48.07148345920022]` in table order. Local receipts remain in `reference-dispute-pulley/`.

This finding invalidates the assumption that valid zero Common alone certifies contact or separation, particularly on detailed fasteners. Every proposed contact proof must survive a cross-check capable of detecting this failure; the six earlier zero-volume disagreements must be independently revalidated before restoring a false-positive headline. No existing measurements or comparison receipts are discarded.

## Released 0.40.0 contact source rechecks

All **22 reported contact-or-separated pairs have completed the capped source recheck**. No shared-interior certificate was found. **Five have bounded negative evidence; seventeen remain disputed**. The five bounded negatives are four Adirondack pairs and OpenArmJIG [2,5]. Thirteen mapped pairs have winding values near one at source ON/ON boundary points, with no sampled point classified IN both source solids. Four folding-table pairs refuse the unchanged strict occurrence matcher: bounds errors 0.184734/0.184756 mm exceed its 0.1-mm limit. No timeout occurred.

All eighteen mapped pairs have valid source solids and a valid, completed Common with exactly zero volume. This does not independently certify exhaustive separation. Each mapped pair was additionally checked using 27-point seed cubes, source classification at 1e-9 mm, fine source-mesh solid-angle winding and outside controls. A positive overlap certificate requires both source IN/IN, both winding inside and more than 0.000001 mm clearance to both source boundaries. Bounded negative sampling is explicitly weaker than a proof over the whole geometry. The desk's complete pair-set claim therefore remains independently unverified.

The local receipts are under `rerun-0400/contact-crosschecks/`. Every row retains its method and matching evidence; disputed rows are not counted as safe.

Bounded source rechecks: 5 contact_or_separated_bounded, 17 disputed.

Each pair uses strict occurrence matching, valid source solids, OCCT Common, source point classification in bounded seed cubes, and independent solid-angle winding of fine source meshes. An overlap certificate also requires more than 0.000001 mm clearance to both source boundaries. Negative sampling is bounded; it does not establish exhaustive separation. Boundary disagreements, invalid sources, mapping refusals and timeouts remain disputed.

| Model | Burr pair | Status | Common mm³ | Source IN/IN samples | Winding IN/IN samples | Interior certificates | Limitation |
|---|---|---|---:|---:|---:|---:|---|
| FreeCAD-library__Adirondack Chair.step | 4/6 | disputed | 0.0 | 0 | 3 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 4/9 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 4/22 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 4/23 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 4/24 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 5/7 | disputed | 0.0 | 0 | 11 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 5/8 | disputed | 0.0 | 0 | 5 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 5/22 | disputed | 0.0 | 0 | 6 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 5/23 | disputed | 0.0 | 0 | 7 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 5/24 | disputed | 0.0 | 0 | 6 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 6/9 | disputed | 0.0 | 0 | 5 | 0 |  |
| FreeCAD-library__Adirondack Chair.step | 7/8 | disputed | 0.0 | 0 | 4 | 0 |  |
| FreeCAD-library__ComputerDesk (100 x 50 x 75 cm WDH).step | 0/1 | disputed | 0.0 | 0 | 11 | 0 |  |
| FreeCAD-library__ComputerDesk (100 x 50 x 75 cm WDH).step | 0/2 | disputed | 0.0 | 0 | 3 | 0 |  |
| FreeCAD-library__ComputerDesk (100 x 50 x 75 cm WDH).step | 1/4 | disputed | 0.0 | 0 | 3 | 0 |  |
| FreeCAD-library__ComputerDesk (100 x 50 x 75 cm WDH).step | 1/5 | disputed | 0.0 | 0 | 12 | 0 |  |
| FreeCAD-library__Wooden Folding Table.step | 0/2 | disputed | — | 0 | 0 | 0 | Ambiguous placement for occurrence 0: 0.18473414499615076, 36.807239182404416 |
| FreeCAD-library__Wooden Folding Table.step | 0/3 | disputed | — | 0 | 0 | 0 | Ambiguous placement for occurrence 0: 0.18473414499615076, 36.807239182404416 |
| FreeCAD-library__Wooden Folding Table.step | 1/2 | disputed | — | 0 | 0 | 0 | Ambiguous placement for occurrence 1: 0.184755730916784, 36.80725960415718 |
| FreeCAD-library__Wooden Folding Table.step | 1/3 | disputed | — | 0 | 0 | 0 | Ambiguous placement for occurrence 1: 0.184755730916784, 36.80725960415718 |
| openarm_hardware__OpenArmJIG.STEP | 2/3 | disputed | 0.0 | 0 | 12 | 0 |  |
| openarm_hardware__OpenArmJIG.STEP | 2/5 | contact_or_separated_bounded | 0.0 | 0 | 0 | 0 |  |

## Released 0.40.0 rerun — in progress

The original 0.39.0 baseline above remains unchanged. A separately installed, checksum-verified 0.40.0 release is rerunning the original 25 models after the gearmotor fix, spike fix and contact proofs. Per-model interference comparison, exact-zero validation of every reported contact-or-separated pair, and unresolved reasons will be reported separately. No follow-up accuracy or contact-safety claim is made before those checks complete.

An early import regression is already measured: Jubilee's left double-pulley corner-bracket assembly loses source face 22181 (`ContradictoryDualParity`, source/synthetic `DuplicateTraversal`), returning `incomplete` with 1/510 faces lost and zero checked pairs. 0.39.0 imported it without face loss and reported a supported `fail` with unresolved pairs. The existing OCCT source reference validates all 23 components. Removal of its coordinate-limit reason is not a gain because the new import stops checking earlier. The exact cause of the new refusal is not established.

### Per-model 0.40.0 checkpoint

Release runs: 25/25 completed (8 `fail`, 16 `incomplete`, 1 `pass`). Verification columns remain provisional. “Complete” is Burr's pair-set claim, not yet certification of agreement; every reported contact remains subject to independent source verification. Empty contact sets require no CAD calculation. The table's zero counts never turn a pending comparison into agreement.

| Model | 0.39.0 coordinate limit | 0.40.0 verdict | Pair set complete | Interferences matched / extra / missing | Contact Common zeros / reported (source proof separate) | Unresolved pairs and reasons |
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

The sole absent pair is pulley Burr [18,22], bracket against M4 shoulder screw. A fresh priority audit strictly maps **both released versions** to the same OCCT [0,1], with both source solids valid and valid completed exact Common volume **0.13749159683157497 mm³** (threshold 1e-6 mm³), bounds/surface errors below 0.000008 mm. The 0.40.0 model is `incomplete` with zero pairs checked: loss of unrelated M5 screw face 22181 blocks the whole check. It is not called safe or given a pass verdict, but real positive evidence is absent from all pair lists. Original 23-part STEP is the confirmed repro. A source-preserving reduction retaining bracket, M4 screw and M5 screw placements (`12-pulley-positive-and-import-blocker.step`, 1,466,783 bytes) is prepared and **not yet validated**.

The priority audit has independently checked **20 of 49 removed findings**: **14 valid positive-volume pairs** and **6 exactly zero-volume Common disagreements**. Thirteen positive pairs are now unresolved (six Bondtech, four pen, three template); the fourteenth is the absent pulley pair behind the incomplete import. Four pairs refused strict placement matching (three Bondtech and one Biped), and 25 Biped pairs remain pending after the 600-second audit cap. All six baseline Common-zero disagreements are now unresolved; non-overlap remains independently unverified. No removed finding entered the contact-or-separated list. Unresolved real overlaps represent reduced usefulness, while the zero-volume pairs now require independent source revalidation. The strict limits were not relaxed. A cached source loader is prepared to avoid repeated STEP imports in the remaining per-pair audit; it does not change the matcher, geometry or tolerances.

Read-only comparison of the released source narrows the explanation. [#60](https://github.com/fraylabs/burr/commit/fe4473a46281465b5f785d1abf1ba5589cc898b8) adds nearby curved-trim deviation to the penetration budget and treats long tangential crossings as sampling uncertainty. That is consistent with new tessellation-limit transitions; diagnostics do not isolate the decisive threshold for each pair. [#55](https://github.com/fraylabs/burr/commit/c8c4eeb) adds a source contact proof after positive-interference testing fails, and leaves the open-mesh refusal gate in place. None of these 49 removed findings entered its contact list. The checker's mesh-closure calculation is unchanged between the two releases, so newly open pairs reflect changed imported meshes rather than a newly added open-mesh gate. Look's pinned revision changed from `58c39f4` to `6b1021c`; this measurement does not isolate which meshing change opened each component. The pulley transition is directly attributed to lost face 22181 and the existing import preflight.


## Regressions vs 0.39.0

The same pinned source files were cold-loaded in the two installed releases. All 25 new release runs are finished, but the new strict comparisons and exact-zero contact validations are still in progress. A drop in raw pair count is not automatically a missed real overlap: the removed pair may have been false, or may now be unresolved. A confirmed false negative still requires a complete Burr pair set and strict exact-source agreement.

| Model | 0.39.0 → 0.40.0 | What is established | Pair-level state |
|---|---|---|---|
| Jubilee left double-pulley corner bracket | 0 → 1/510 faces lost; `fail` → `incomplete`; 1 → 0 reported pairs | source face 22181 now fails with `ContradictoryDualParity`; 23 exact source components valid | checking stops at import; retained-screw reduction 11 prepared, not yet validated |
| Full Jubilee | 2 → 3/9453 faces lost; both `incomplete` | old failed faces 286421/400279 no longer warn; new failed faces 202439/299236/389430, all parity failures | import remains incomplete; not one additional copy of the old same failed face |
| Jubilee passive pen | `fail` → `incomplete`; 4 → 0 reported pairs | old subset independently matched four true pairs; supported fail evidence is no longer reported | fresh strict selected-pair mapping and valid positive Common; all now open-mesh unresolved |
| Jubilee tool template | `fail` → `incomplete`; 3 → 0 reported pairs | old subset independently matched three true pairs; supported fail evidence is no longer reported | fresh strict selected-pair mapping and valid positive Common; all now open-mesh unresolved |
| Biped | 63 → 32 reported pairs; both `fail` | five old interference/Common-zero disagreements; now tessellation unresolved | one strict refusal, 25 removed pairs pending; full pair agreement not certified |
| Bondtech extruder | 18 → 9 reported pairs; both `fail` | six valid positive Common pairs now open-mesh unresolved | three tessellation-unresolved pairs refuse strict placement matching |

The gearmotor's 5 → 4 reported pairs removes the Metal–Yellow Common-zero disagreement; independent non-overlap validation is pending. The original model still reports the four prior positive Common pairs, and leaves Metal–Yellow unresolved below tessellation resolution. Strict comparison and a fresh 0.40.0 run of the two-part repro remain pending. It is kept separate from the regression candidates.

The desk is the only one of the 11 former coordinate-limit models now reporting a complete pair set. Four no longer list the coordinate reason, but one is the pulley import failure; removing a reason by stopping checking earlier is not a gain. There are 22 contact-or-separated pairs reported across four models. Every one still requires an independently valid OCCT Common with exactly zero volume before it is described as verified.

## Classification and reference limits

The 25 completed measurement jobs yield raw Burr outcomes **11 `fail`, 13 `incomplete`, 1 `pass`**. Of the 17 comparison records, eight have full occurrence mapping and a full reference scan, five certify a subset, and four refuse comparison. Eight more comparisons are pending. These records match **57 reported positive pairs** to OCCT and contain **one extra pair against the Common reference**. The separate removed-pair audit also records five Biped Common-zero disagreements despite its incomplete full comparison. Together there are six Common-zero disagreements across two models; their former false-positive classification is suspended. These are counts within verified scopes, not a precision or recall estimate for unverified pairs.

Across the 25 references, 17 completed a full pair scan, six used the existing positive-witness shortcut, and two timed out. Five references contain invalid source components. The webcam's sole raw Burr `pass` remains unresolved against an invalid reference; it is not a confirmed false pass. Bed and cell reference timeouts are not Burr timeouts. Source counts are component occurrences, with compounds potentially containing several solids.

Use exclusive classes: correct, incomplete, false positive, false negative, crash, timeout. A confirmed extra pair takes precedence over an incomplete pair scan. A missing exact positive is a confirmed false negative only when Burr claims a complete pair set and strict full occurrence mapping succeeds. Partial OCCT scans, ambiguous source matching and unverified pairs remain incomplete; they are not silently counted as correct or false.

“Correct” means fully verified occurrence and pair agreement. A supported `fail` verdict can still be incomplete because Burr or the reference did not finish verifying every pair. Timings are cold-cache release viewer HTTP time (including import and viewer preparation), subsequent check time and OCCT import time; RSS includes the complete respective stage. See README.md for stage sampling and reference shortcut details.

## Which fixes could make incomplete models conclusive?

The following fix ranking preserves the previous **23-model subset**, excluding the two disputed Common-zero models and both pending measurements. The current classification includes those two disputed models as incomplete; this historical subset is kept explicit until source revalidation finishes. The single-fix candidate count below includes only models whose logged Burr reasons contain **that reason alone**. A fix must resolve every affected pair and avoid introducing wrong pairs. These are candidates for a conclusive Burr check, not promised fully verified successes: reference limits, occurrence matching and pending comparisons remain separate blockers. Multiple-reason models do not count as a single-fix win.

| Priority by single-fix candidates | Reason | Single-fix candidates / 23 | Candidates with full reference scan and strict full mapping already available | All affected / 23 | Minimal validated repro |
|---:|---|---:|---:|---:|---|
| 1 | `step_faces_lost` | 7 | 1 (PCB) | 7 | 04, board + LED; 2 parts, 291,185 bytes |
| 2= | `open_component_mesh` | 4 | 1 (expansion card) | 9 | 09, two jig occurrences; 2 parts, 90,297 bytes |
| 2= | `below_coordinate_resolution` | 4 | 2 (Adirondack chair, desk) | 9 | 10, Metal + Plastic; 2 parts, 82,830 bytes |
| 4 | `assembly_structure_lost` | 1 | 0; source scan complete, mapping refused | 1 | 05, door + wire; 2 leaves, 282,598 bytes |
| 5 | `below_tessellation_resolution` | 0 | 0 | 3 | 07, hinge axis + rivets; 3 parts, 109,112 bytes |

The seven face-loss candidates are full laptop, battery, Jubilee, bed, PCB, OpenArm 2.0 and OpenArm Cell. They do **not** share one proven meshing defect: the diagnostic mechanisms differ, two references timed out and some source compounds are invalid. Seven is the potential gain from resolving the whole class, not from fixing PCB parity alone.

The four open-mesh-only candidates are expansion card, tool template, passive pen and camera calibration tool. Three currently have incomplete occurrence mapping, so a closure fix alone cannot yet certify their pair accuracy. The four coordinate-only candidates are Adirondack chair, wooden folding chair, wooden folding table and desk; folding chair matching is refused and folding table mapping is partial.

Five other models need combined fixes: pulley bracket and OpenArmJIG have both coordinate and open-mesh reasons; hinge, printable case and Bondtech extruder have coordinate, open-mesh and tessellation reasons. Fixing open meshes plus coordinate limits would remove the logged product blockers from 10 models (the eight single-reason candidates plus pulley bracket and jig); tessellation would still block the other three. The two remaining unverified models, webcam and ooze wiper, already have complete Burr pair sets and require reference/matching resolution rather than removal of a logged incomplete reason. All counts are provisional until reruns establish the new behavior.

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
| open_robot_actuator_hardware__biped_6dof_v1.STEP | incomplete | 345 / 345 | 0 / — | fail (partial) | fail (partial) | —; 5 Common-zero disagreements (3 bounded negatives; 2 disputed source checks) | 1.682 | 36.356 | 777.547 | 48.774 | 50.090 | 689.438 |
| kicadStepUpMod__demo.step | incomplete | 13 / 13 | 1 / 401 | incomplete (partial) | fail | 0 / 0 / 2 | 0.161 | 0.060 | 58.766 | 3.467 | 4.152 | 475.875 |
| FreeCAD-library__Adirondack Chair.step | incomplete | 36 / 36 | 0 / — | fail (partial) | fail | 38 / 0 / 4 | 0.070 | 0.171 | 25.141 | 2.426 | 3.343 | 456.844 |
| FreeCAD-library__wooden folding chair.step | incomplete | 20 / 20 | 0 / — | fail (partial) | fail | refused | 0.063 | 0.116 | 25.953 | 2.303 | 2.684 | 454.219 |
| FreeCAD-library__Wooden Folding Table.step | incomplete | 14 / 14 | 0 / — | incomplete (partial) | fail | 0 / 0 / 4 (partial mapping) | 0.062 | 0.056 | 22.203 | 2.254 | 2.564 | 453.734 |
| FreeCAD-library__Double glass doors with handles and transom.step | incomplete | 1 / 3 | 0 / — | incomplete (partial) | pass | refused | 0.066 | 0.057 | 22.781 | 2.215 | 2.476 | 451.844 |
| FreeCAD-library__Yellow_gearmotor_L.step | incomplete | 6 / 6 | 0 / — | fail (partial) | fail | 4 / 1 / 0 (independent source recheck disputed) | 0.068 | 0.282 | 30.484 | 2.230 | 2.683 | 458.000 |
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

## Original corpus contact audit — in progress

The reviewed Checks candidate's external reference has 100 Common-zero contact receipts across the original corpus: Faze4 (93), automation control unit (2), and pick/place arm (5). Burr also records analytic support and precision/volume bounds; the external OCCT corroboration used Common alone. These historical claims are now being rechecked with the same capped independent methods. They are not a fresh released 0.40.0 measurement and do not enter the held-out denominator.

All three source hashes match both the original source manifest and the contact receipts. Original reports and scenes are preserved locally. Only scene display labels for the control unit and pick/place arm were replaced by their own report's canonical definition names; bounds, transforms and surface samples remain unchanged. Current receipts and explicit provenance stay under `original-corpus-contact-rechecks/`. Any shared-interior certificate is reviewed against the pair's declared precision and volume bound and reported immediately.

Early Faze4 receipts expose a distinction between literal zero and permitted source-scale shared interior: Board/resistor samples are classified IN both with winding near one despite empty Common. Pair [0,8] has source-boundary clearances 1.7484555e-8/1.7397133e-8 mm, consistent with the report's declared maximum shared depth (~3.498e-8 mm), below its 1e-7-mm source tolerance. This does not establish a violation of the reported precision/volume bound. Such sub-certificate interior candidates remain disputed and retain their exact margins; the robust certificate requires more than 1e-6-mm clearance.

<!-- original-source-rechecks-start -->

Snapshot of the running audit after the Owner-authorized release timing window. The source driver resumed from 55 saved receipts; pending rows have no completed independent source check. The completed normal tie-breaker needs replay on 38 preliminary historical receipts, thirteen mapped released-contact disputes and three baseline disputes. Raw receipts are retained, and four strict matching refusals remain separate. Contact within tolerance is a bounded witness result, not literal zero or exhaustive separation.

The capped per-pair controller supports a stop file, checks it before starting another reference job or while waiting for the shared lock, and exits paused with owned-lock cleanup. It was smoke-checked to stop without starting a reference child, acquiring a lock or writing a completion claim. The timing pause preserved all completed source receipts; no ordinary measurements resumed.

Bounded source rechecks: 20 contact_within_tolerance_bounded, 36 disputed, 44 pending.

Each pair uses strict occurrence matching, valid source solids, OCCT Common, source point classification in bounded seed cubes, and independent solid-angle winding of fine source meshes. An overlap certificate also requires positive distance to both source boundaries. Negative sampling is bounded; it does not establish exhaustive separation. Boundary disagreements, invalid sources, mapping refusals and timeouts remain disputed.

The maximum sampled clearance is the largest sampled distance to the nearer of the two source boundaries among points classified IN by both source solids and inside by both winding checks. It is local evidence, not a measured total overlap depth or volume. The declared depth and volume bounds come from the tested Burr report; a sampled clearance below them does not verify those global bounds. A dash means no candidate clearance was recorded, not zero clearance.

Method 1 uses Common, seed cubes and winding. Methods 2–4 retain preliminary normal-probe receipts. Method 5 probes both face-normal signs and adjacent-face bisectors at 1e-6, 1e-4, 1e-3 and 1e-2 mm, verifies which steps enter the originating source solid, and checks those points against the other solid and winding. It records boundary distances for every source IN/IN point, including winding disagreements. Shared interior within 1e-6 mm of either boundary is treated as tolerance contact at the sampled witness; only deeper shared interior corroborated by winding yields an overlap certificate. These remain bounded local checks; earlier disputed rows without the completed tie-breaker need rechecking.

| Model | Burr pair | Status | Method | Common mm³ | Source IN/IN samples | Winding IN/IN samples | Verified inward steps | Interior certificates | Max sampled clearance mm | Declared max depth mm | Declared max volume mm³ | Limitation |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/1 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | — | 3.498388000744728e-08 | 4.477937240843201e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/2 | disputed | 1 | 0.0 | 2 | 5 | 0 | 0 | — | 3.4983880007447185e-08 | 4.4779377302161485e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/3 | disputed | 1 | 0.0 | 2 | 4 | 0 | 0 | — | 3.498388000744731e-08 | 4.4779368849872485e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/4 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | — | 3.498388000744728e-08 | 4.4779372408313886e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/6 | disputed | 1 | 0.0 | 12 | 18 | 0 | 0 | — | 1.7499324656011864e-08 | 2.659897839216933e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/8 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.4779372407087713e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/9 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.4779367513953614e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/10 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240711754e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/11 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.477936751401041e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/12 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240714617e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/13 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.4779367514068155e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/14 | disputed | 1 | 0.0 | 2 | 6 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240717243e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/15 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.477936751412853e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/16 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.4779372407202255e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/17 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.477936751419105e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/18 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240723328e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/19 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.4779367514255475e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/20 | disputed | 1 | 0.0 | 2 | 6 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240726549e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/21 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.477936751431275e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/22 | disputed | 1 | 0.0 | 2 | 6 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240729413e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/23 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.7397132556246827e-08 | 3.498388000744732e-08 | 4.477936751437798e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/24 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240732674e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/25 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.7397132556246827e-08 | 3.498388000744732e-08 | 4.477936751444194e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/26 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.4779372407358956e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/27 | disputed | 1 | 0.0 | 2 | 5 | 0 | 0 | 1.7397132553652203e-08 | 3.498388000744678e-08 | 4.4779387088197585e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/28 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240739595e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/29 | disputed | 1 | 0.0 | 2 | 5 | 0 | 0 | 1.7397132553652203e-08 | 3.498388000744678e-08 | 4.47793870882644e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/30 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.477937240742935e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/31 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.7397132556247138e-08 | 3.498388000744732e-08 | 4.4779367513722135e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/32 | disputed | 1 | 0.0 | 2 | 7 | 0 | 0 | 1.7397132552425845e-08 | 3.498388000744728e-08 | 4.4779372406996836e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/33 | disputed | 1 | 0.0 | 2 | 10 | 0 | 0 | 1.739713255624745e-08 | 3.498388000744732e-08 | 4.477936751378418e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/34 | disputed | 2 | 0.0 | 10 | 18 | 44 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.477937240702805e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/35 | disputed | 2 | 0.0 | 10 | 26 | 44 | 0 | 1.748451172543306e-08 | 3.498388000744732e-08 | 4.4779367513839066e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/36 | disputed | 2 | 0.0 | 10 | 18 | 40 | 0 | 1.748451172543303e-08 | 3.498388000744728e-08 | 4.477937240705788e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/37 | disputed | 3 | 0.0 | 10 | 26 | 40 | 0 | 1.748451172543306e-08 | 3.498388000744732e-08 | 4.477936751355151e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/38 | disputed | 3 | 0.0 | 10 | 19 | 40 | 0 | 1.748451172543303e-08 | 3.498388000744728e-08 | 4.477937240691211e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/39 | contact_within_tolerance_bounded | 4 | 0.0 | 10 | 26 | 64 | 0 | 1.748451172543306e-08 | 3.498388000744732e-08 | 4.47793675136064e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/40 | contact_within_tolerance_bounded | 4 | 0.0 | 10 | 19 | 64 | 0 | 1.748451172543303e-08 | 3.498388000744728e-08 | 4.477937240693956e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/41 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 26 | 64 | 0 | 1.748451172543306e-08 | 3.498388000744732e-08 | 4.477936751366415e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/42 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.4779372406968196e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/43 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.4779372406846594e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/44 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 17 | 64 | 0 | 1.7484511669921776e-08 | 3.4983880007447185e-08 | 4.477937730028136e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/45 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.477937240678813e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/46 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 18 | 64 | 0 | 1.7484511669921776e-08 | 3.4983880007447185e-08 | 4.477937730025282e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/47 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 20 | 64 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.4779372406723694e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/48 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 16 | 64 | 0 | 1.7484511614410625e-08 | 3.4983880007447185e-08 | 4.477937730022061e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/49 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 20 | 64 | 0 | 1.748451172543303e-08 | 3.498388000744728e-08 | 4.477937240710789e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/50 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 20 | 64 | 0 | 1.7484511669921776e-08 | 3.4983880007447185e-08 | 4.47793773004139e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/51 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451172543303e-08 | 3.498388000744728e-08 | 4.477937240704657e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/52 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 18 | 64 | 0 | 1.7484511669921776e-08 | 3.4983880007447185e-08 | 4.4779377300383596e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/53 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451172543303e-08 | 3.498388000744728e-08 | 4.477937240698858e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/54 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.7484511614410625e-08 | 3.4983880007447185e-08 | 4.477937730035185e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/55 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.477937240754629e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/56 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 16 | 64 | 0 | 1.7484511614410625e-08 | 3.4983880007447185e-08 | 4.47793773017224e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/57 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 17 | 64 | 0 | 1.748451172543306e-08 | 3.498388000744731e-08 | 4.477936884898736e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/58 | contact_within_tolerance_bounded | 5 | 0.0 | 10 | 19 | 64 | 0 | 1.748451166992188e-08 | 3.498388000744728e-08 | 4.477937240742816e-08 |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/59 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/60 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/61 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/62 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/63 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/64 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/65 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/66 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/67 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/68 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/69 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/70 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/71 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/72 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/73 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/74 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/75 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/76 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/77 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/78 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/103 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/106 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/109 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/112 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/115 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/118 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/121 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/123 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/126 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/128 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/129 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/130 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/131 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/132 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/133 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/134 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 0/135 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__automation_control_unit.step | 0/1 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__automation_control_unit.step | 12/13 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__pick_place_arm.step | 0/16 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__pick_place_arm.step | 1/2 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__pick_place_arm.step | 2/3 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__pick_place_arm.step | 3/4 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |
| text-to-cad-assembly__pick_place_arm.step | 3/5 | pending | — | — | 0 | 0 | 0 | 0 | — | — | — |  |

<!-- original-source-rechecks-end -->

## Candidate 0.40.1 pulley contact gate

The frozen mesh-worker candidate (binary SHA-256 `69ac75ecb3c7a3eb1153c3d868c7cc163904986556e24d4986afb65099bbff6d`) reports 12 contact-or-separated pairs on the same held-out pulley source. This is a release-candidate gate, separate from the installed-release denominator. All 12 pairs passed the unchanged strict source occurrence matcher and had valid source solids and completed, valid Common with exactly zero volume. The bounded seed cubes, source classification, independent winding and inward-normal tie-breaker found no robust shared-interior certificate under the requested 1e-6 mm clearance rule: three bounded contact-or-separated results and nine bounded tolerance-contact results. There were no matching refusals, invalid sources or 150-second timeouts.

Pair 12/14 (M3 standoff / reinforcement plate) initially remained disputed because an underlying-surface normal projected outside the trimmed face. A separately capped follow-up used an actual closest point on the trimmed face, retained the same on-face check and source-verified entry rules, and returned bounded tolerance contact. The original receipt is retained unchanged. Both drivers exited 0 and released their owner-file locks normally. The Owner accepted this qualified gate result; the candidate is not counted as an additional held-out model.

Finite witness checks do not establish exhaustive separation or verify global overlap-depth or volume bounds. All receipts and input hashes remain local under `candidate-0401-pulley-contacts/`; `review.json` records which original or follow-up receipt supports each final row.

| Burr pair | Final bounded result | Common mm³ | Source IN/IN samples | Winding IN/IN samples | Verified inward steps | Follow-up |
|---|---|---:|---:|---:|---:|---|
| 0/7 | contact_or_separated_bounded | 0 | 0 | 0 | 56 | — |
| 0/10 | contact_within_tolerance_bounded | 0 | 0 | 25 | 56 | — |
| 2/18 | contact_within_tolerance_bounded | 0 | 0 | 27 | 56 | — |
| 3/14 | contact_within_tolerance_bounded | 0 | 0 | 25 | 52 | — |
| 3/18 | contact_within_tolerance_bounded | 0 | 0 | 26 | 48 | — |
| 6/10 | contact_within_tolerance_bounded | 0 | 0 | 27 | 56 | — |
| 6/18 | contact_within_tolerance_bounded | 0 | 0 | 24 | 52 | — |
| 11/18 | contact_or_separated_bounded | 0 | 0 | 0 | 64 | — |
| 12/14 | contact_within_tolerance_bounded | 0 | 0 | 8 | 52 | trimmed-face closest point; original dispute retained |
| 12/18 | contact_or_separated_bounded | 0 | 0 | 0 | 48 | — |
| 15/18 | contact_within_tolerance_bounded | 0 | 0 | 28 | 56 | — |
| 18/21 | contact_within_tolerance_bounded | 0 | 0 | 31 | 56 | — |
