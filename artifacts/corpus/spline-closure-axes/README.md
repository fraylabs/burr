Switchwire import returns from a timeout above 120 seconds to 7.12 seconds. The same fix reduces open boundaries on the arcade button's Dome. Reversed STEP spline faces now carry closure flags in the converted evaluator's parameter axes. The STEP converter swaps U and V when it reverses a spline surface. Previously the tessellator received the original surface flags, so it could fail to recognize and wrap the converted closed axis. Face-specific flags now follow that conversion; unchanged faces retain their source flags.

A synthetic regression first failed on missing face-specific metadata. It now checks both face senses and the physical evaluator's closed axis and wrapping. The focused lattice suite passes 23 tests. Tessellation cache revision changes from 5 to 6.

The real arcade button still has open valid parts. This change reduces those boundaries without claiming a complete assembly check:

| Arcade part | Open edges before → after | Non-manifold edges before → after |
|---|---:|---:|
| Base | 0 → 0 | 0 → 0 |
| Holder (OCCT-invalid) | 0 → 0 | 1 → 1 |
| Actuator | 0 → 0 | 0 → 0 |
| Spring | 29 → 29 | 0 → 0 |
| Dome | 240 → 121 | 0 → 0 |

Arcade remains incomplete before and after. Its own reports each record seven candidate pairs, ten checked pairs, zero confirmed pairs and seven unresolved pairs; [arcade-status.json](arcade-status.json) archives these distinct metrics and the unresolved-pair IDs. Spring and Dome need further boundary diagnosis; Holder remains an invalid source solid.

All ten protected models were compared against the existing OCCT pair references using fresh scene dumps and the unchanged placement, identity and surface-sample checks. They retain 64 true pairs and zero false pairs. Bowden retains its same nine confirmed OCCT-positive pairs, while all nine previously false reported pairs stay unresolved. The Bowden reference covers those eighteen reported pairs; full pair completeness remains unknown. The attached pair-summary.json contains per-model counts, not CAD coordinates.

The baseline is Look d1583a7 with the merged uncertainty review policy. The pair comparisons were rerun using fresh candidate scene dumps; interference code remains unchanged after rebasing onto the merged binary viewer and example updates. Source screening also identifies reversed closed splines in V0, V2, Switchwire and the full robot. Additional candidate runs completed for all four: each stays incomplete with zero confirmed pairs. Full robot retains 17 lost faces, V0 retains 6, V2 retains 4, and Switchwire reports 46. Those lost faces continue to block a conclusive check. The attached additional-models.json records the candidate measurements.

Fresh released-binary measurements isolate import from viewer generation. Each released binary was installed using the public one-line installer into a separate folder. Each request used a fresh server and cache, with one model running under the shared build lock. These three printers refuse the check on lost import faces, so the cold `/api/checks` request completes before mesh check preparation. No viewer HTML was requested. Values are single wall-clock observations on the shared Mac, rather than statistical estimates.

| Printer | Released 0.37.0 | Released 0.38.0 | Released 0.38.1 | This fix |
|---|---:|---:|---:|---:|
| Switchwire | 6.77 s | >120 s, stopped | >120 s, stopped | 7.12 s |
| Voron V0 | 13.04 s | — | 13.56 s | 13.36 s |
| Voron V2.4 | 33.98 s | — | 38.77 s | 36.38 s |

The candidate is the published Look pin below, built locally in release mode; the other columns use installed release assets. A timeout is not a completed import. The fresh Switchwire stage measurement puts 5.72 seconds in tessellation, 0.28 seconds in STEP parsing, 0.45 seconds in table construction and 0.17 seconds in scene compilation (7.83 seconds total in this separate run). The existing shared-edge refinement is retained. The synthetic closure regression checks the inverted physical axis, rather than setting a load-time threshold.

A private counterfactual build restores only the original surface-axis flags before constructing the policy surface. Its control finishes in 9.87 seconds with diagnostic logging; restoring the old flags times out at 30 seconds. Source face 639696 reverses a rational spline whose closed source U axis becomes evaluator V. The wrong flags fail to certify that axis, disabling the periodic chart handling and its ordinary-axis range guard. A spurious inverse then escapes the native ordinary interval and inflates the interior grid to 17 × 6,933 candidate points. Correct face-specific flags keep that grid at 5 × 5. The costly stage is the resulting grid insertion and trimming. Disabling quotient subdivision alone stays fast, so that subdivision implementation is not independently responsible. All diagnostic hooks were removed before the final local checks.

The pair results are retained alongside the speed recovery: Switchwire still refuses a conclusive check on 46 lost faces, versus 51 in 0.37.0. V0 retains the candidate's six lost faces and V2.4 its four. The protected pair gate and Bowden gate above were rerun, with unchanged 64/0 and 9/0 results. All 177 previously closed source shells in the seven-model topology archive remain closed and manifold; all 185 archived source shells were matched by source face IDs. [import-performance.json](import-performance.json) records timing provenance and [closed-shells.json](closed-shells.json) records those topology counts without CAD coordinates.

Burr pins Look fix/spline-closure-axes at 45f1d77986a59aef48357c6aa0ad4c5edff28e72. The full npm run check passed against this published Git pin: strict Clippy, 77 tests and the viewer proof. The rebased binary viewer and its implementation fingerprint remain in place; its cache key advances so previous tessellations are regenerated.

Known limitation: the broader V2.4 topology audit found a new non-manifold edge in Meanwell LRS-200-24 Body2, on source face 1797059. The source solid changes from 0 open / 0 non-manifold edges to 0 open / 1 non-manifold edge (4,660 → 4,662 triangles). Opposite seam-side chords coincide in the periodic chart; the defect exists before float conversion. Its separate seam-sampling fix is deferred. This means the seven-model archive above passes, but the broader closed-mesh audit has this explicit exception.

| V2.4 gate | Without axis fix → with axis fix |
|---|---|
| Verdict | incomplete → incomplete |
| Lost import faces | 4 → 4 |
| Candidate / checked / confirmed pairs | 0 / 0 / 0 → 0 / 0 / 0 |
| Meanwell Body2 open / non-manifold edges | 0 / 0 → 0 / 1 (known limitation) |

The complete V2.4 check reports are identical, including the refusal reason and empty pair lists; [v24-report-equivalence.json](v24-report-equivalence.json) archives that comparison. No V2.4 interference verdict is claimed.
