Reversed STEP spline faces now carry closure flags in the converted evaluator's parameter axes. The STEP converter swaps U and V when it reverses a spline surface. Previously the tessellator received the original surface flags, so it could fail to recognize and wrap the converted closed axis. Face-specific flags now follow that conversion; unchanged faces retain their source flags.

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

The baseline is Look d1583a7 with the merged uncertainty review policy, measured locally using the frozen review binary. Source screening also identifies reversed closed splines in V0, V2, Switchwire and the full robot. Additional candidate runs completed for all four: each stays incomplete with zero confirmed pairs. Full robot retains 17 lost faces, V0 retains 6, V2 retains 4, and Switchwire reports 46. Those lost faces continue to block a conclusive check. The attached additional-models.json records the candidate measurements.

Switchwire loaded in 7.82 seconds on this candidate. The Owner integration worker's recorded baseline timed out after 600 seconds; its baseline Cargo pin and Look checkout both identify d1583a7. This is a reused timeout comparison, not a newly executed baseline or proof that every timing difference comes from the axis fix.

Burr pins Look fix/spline-closure-axes at 45f1d77986a59aef48357c6aa0ad4c5edff28e72. The full npm run check passed against this published Git pin: strict Clippy, 71 tests and the viewer proof. Burr also advances its viewer cache key so cached geometry from the previous tessellator is regenerated.
