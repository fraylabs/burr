Faze4 keeps all nine confirmed problems. Of its 126 uncertain pairs, 93 now have source support proving no interference above the authored STEP precision; 33 remain unresolved in two expandable groups. The Contacts / separated category starts collapsed; the proof does not assert that trimmed faces touch. Selecting a group highlights every occurrence, and its expanded rows preserve individual pair selection. Generic solid labels use the nearest named ancestor or filename, with sibling indexes.

The baseline is main 0.39.0 (`b0bde98`) after #49 and #45, with Look `58c39f4`. The candidate is now rebased onto main `62e7a2d`, including #60’s curved-contact guard and #61’s V2.4 spike fix, with Look `6b1021c`. The corpus counts remain unchanged after both the earlier `bd72ace` rebase and this rebase. Every one of the 21 corpus models and six repros completed before and after without a time or memory cap. Counts below are **real / contact-or-separated / unresolved**. An incomplete result with zero pairs remains incomplete.

| Model | Before | After | Result |
|---|---:|---:|---|
| 01-assembly-pds-dollar.step | 0 / 0 / 0 | 0 / 0 / 0 | pass |
| 01-assembly-pds-empty-strings.step | 0 / 0 / 0 | 0 / 0 / 0 | pass |
| 02-edge-conversion-face-1073.step | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| 02-edge-conversion-minimal-face.step | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| 03-negative-torus-face.step | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| 04-mesh-contact-pair.step | 0 / 0 / 0 | 0 / 0 / 0 | pass |
| Adafruit_CAD_Parts__1185_Massive_Arcade_Button_100mm.step | 0 / 0 / 7 | 0 / 0 / 7 | incomplete |
| Adafruit_CAD_Parts__1400_Push_Button_Power_Switch.step | 11 / 0 / 13 | 11 / 0 / 13 | fail |
| Faze4-Robotic-arm__Faze4_dist_v2_STEP.step | 9 / 0 / 126 | 9 / 93 / 33 | fail |
| TradRack__tr-lower-carabiner-distribution-board.STEP | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| TradRack__tr-toolhead-board.STEP | 6 / 0 / 0 | 6 / 0 / 0 | fail |
| Voron-0__Fly_Gemini_V2_Din_Mount_2pc.step | 7 / 0 / 7 | 7 / 0 / 7 | fail |
| Voron-0__Manta_M4P_Din_Mount_2pc.step | 0 / 0 / 0 | 0 / 0 / 0 | pass |
| Voron-0__MiniSB_Bowden.step | 9 / 0 / 76 | 9 / 0 / 76 | fail |
| Voron-0__MiniSB_adxl_mount_adafruit_19mm_c_c.step | 0 / 0 / 13 | 0 / 0 / 13 | incomplete |
| Voron-0__V0.2R1_Master_Assembly_v63.step | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| Voron-2__Voron_2.4r2_Assembly.step | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| Voron-Switchwire__Switchwire_Assembly_v1_STEP.step | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| apollo-3d-enclosure__corner__assembly.step | 0 / 0 / 0 | 0 / 0 / 0 | pass |
| apollo-3d-enclosure__flat__assembly.step | 0 / 0 / 0 | 0 / 0 / 0 | pass |
| openamr-platform-hw__MMP.00.00.00.000_full_assembly.STEP | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| openamr-platform-hw__MMP.01.00.00.000_Cover_assembly.STEP | 16 / 0 / 25 | 16 / 0 / 25 | fail |
| openamr-platform-hw__MMP.02.00.00.000_Base_assembly.STEP | 16 / 0 / 26 | 16 / 0 / 26 | fail |
| openamr-platform-hw__MMP.03.00.00.000_Wheel_assembly.STEP | 0 / 0 / 0 | 0 / 0 / 0 | incomplete |
| openamr-platform-hw__MMP.04.00.00.000_Center_bracket_assembly.STEP | 8 / 0 / 8 | 8 / 0 / 8 | fail |
| text-to-cad-assembly__automation_control_unit.step | 12 / 0 / 2 | 12 / 2 / 0 | fail |
| text-to-cad-assembly__pick_place_arm.step | 3 / 0 / 5 | 3 / 5 / 0 | fail |

The non-interference proof reads hash-matched source BREP geometry and validates occurrence correspondence against the imported scene. It bounds each supported face in double precision and finds a separating plane. Acceptance requires both an arithmetic overlap-depth bound within the declared source tolerance and an outward-rounded common-volume bound at most 0.000001 mm³. Confirmed mesh witnesses retain priority. Unsupported carriers, ambiguous units or placements, incomplete shells, incoherent planar trims and unsafe arithmetic refuse contact.

Browser evidence is kept locally in the Burr Checks evidence directory, under `evidence/`. The paths below are relative to that directory:

| Model | Before screenshot | After screenshot |
|---|---|---|
| Faze4 | before-faze4/workbench-solid.png | after-faze4/workbench-solid.png |
| Automation control unit | before-control-unit/workbench-solid.png | after-control-unit/workbench-solid.png |
| Robot cover | before-robot-cover/workbench-solid.png | after-robot-cover/workbench-solid.png |

The baseline Faze4 panel repeats 126 unresolved rows; the candidate lists nine findings first, then two unresolved groups and a collapsed contact section. The control unit replaces SOLID labels with AirPump, PneumaticCylinder and TerminalBlock names and indexes; its two uncertain pairs become proven non-interference pairs. The cover preserves 16 findings and 25 unresolved pairs, with full names and groups. All six screenshots were inspected. Chrome checks also verified first-render collapsed contacts, all ten occurrences of the nine-pair unresolved group, all 85 occurrences of the 84-pair resistor contact group, and the two-occurrence pair view. Group and selected-pair frames, X-ray frames and exported PNGs are kept alongside the six primary screenshots.

The final 0.39.1 exact gate passes: ten protected models **64 true / 0 false**, Bowden **9 true / 0 false**, and the importer-protected Base and Center assemblies **24 true / 0 false**. Every before-build confirmed pair was retained. No exact-positive pair entered the contact-or-separated category. Bowden replays its 18 historical pairs and every current reported pair; full Bowden candidate completeness remains unknown.

Every contact-or-separated pair on every corpus input was checked individually with a fresh, valid OCCT Common. **All 100 have exactly zero common volume**, including all 93 on Faze4. The [individual pair results](contact-occt.json) record source hashes, Burr and OCCT occurrence indexes, component names, volumes and precision bounds. The [full model summary](summary.json) records all 27 counts and gate scopes.

| Model | Contact / separated pairs | Fresh OCCT checks | OCCT-positive contacts | Largest common volume (mm³) |
|---|---:|---:|---:|---:|
| Faze4 | 93 | 93 | 0 | 0 |
| Automation control unit | 2 | 2 | 0 | 0 |
| Pick/place arm | 5 | 5 | 0 | 0 |
| Base assembly | 0 | 0 | 0 | — |
| Center bracket assembly | 0 | 0 | 0 | — |
| Other 22 corpus inputs (listed above) | 0 | 0 | 0 | — |

The full local `npm run check` passes all 90 Rust tests and the viewer proof. The final Chrome captures include the requested panel polish: the group action matches the clear action, the resolution note stays with Unresolved, Contacts has a separate explanation, and selected X-ray occurrences are opaque and vivid against faint context.

The report uses `source_non_interference` for this category. The existing `contact_pairs` JSON collection holds its pairs; their messages explicitly permit separation and state only the proven bound on shared interior. Zero Common volume establishes non-interference; it does not independently establish zero separation.

The fresh `62e7a2d` rebase gate retains 97 true / 0 false, including protected 64/0, Bowden 9/0, Base 16/0 and Center 8/0. All 100 contact-or-separated pairs again have fresh, valid OCCT Common volume exactly zero. The original gearmotor retains its four exact-positive pairs and no false pair; its contact repro and corpus2 repros 08–10 remain incomplete with no findings. The gearmotor contact and 0.5 mm real-overlap regressions are retained. [Rebase evidence](rebase-62e7a2d.json) records the fresh gate, binary hashes and inspected Chrome group selection. The local screenshot is `faze4-unresolved-group-xray.png`: Board × LTV-817S selected, ten occurrences, X-ray active, contacts collapsed and no browser or WebGL errors.

Paired import-only timings against exact 0.39.1 (`bd72ace`) pass the owner’s revised shared-Mac rule: median no more than 10% slower, and candidate samples not wholly above baseline on every block. Six runs per binary use balanced baseline/candidate/candidate/baseline blocks, reversing the middle block.

| Model | 0.39.1 median | Checks candidate median |
| --- | ---: | ---: |
| Voron-0__V0.2R1_Master_Assembly_v63.step | 14.345703 s | 14.644697 s |
| Voron-2__Voron_2.4r2_Assembly.step | 39.015956 s | 40.650448 s |
| Voron-Switchwire__Switchwire_Assembly_v1_STEP.step | 7.237767 s | 7.146498 s |
