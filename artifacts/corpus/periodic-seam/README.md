# Keep periodic seam chords manifold

The V2.4 Meanwell power supply has a full-period NURBS face whose coarse interior sampling connected one point to both copies of its seam. Those two UV edges became the same physical edge with four incident triangles. The face had no missing triangles or boundary gap; the edge was non-manifold.

Look now splits interior sampling intervals spanning half a certified period into smaller intervals. It uses the certified lattice period, including source-certified NURBS periods that the native evaluator does not expose. Shared BREP edge samples stay unchanged. The synthetic cylinder regression has no native period accessor and fails before the fix with the same four-incident physical edge.

| Measurement | Before seam fix | This change |
| --- | ---: | ---: |
| Meanwell LRS-200-24 boundary edges | 0 | 0 |
| Meanwell LRS-200-24 non-manifold edges | 1 | 0 |
| Meanwell LRS-200-24 triangles | 256,332 | 256,336 |
| Protected ten models, OCCT true / false pairs | 64 / 0 | 64 / 0 |
| Combined integration gate, true / false pairs | 97 / 0 | 97 / 0 |
| Bowden, confirmed true / false pairs | 9 / 0 | 9 / 0 |

The Meanwell counts come from the full V2.4 import, preserving the printer's global tessellation tolerance. Its prepared mesh is now closed and manifold. The protected verdicts and pair counts remain unchanged; six known cover overlaps and one Fly overlap still remain unresolved. Bowden's reference covers only 18 previously reported pairs: all nine true pairs stay confirmed and the nine former false pairs stay unresolved. Complete Bowden pair coverage remains unknown.

The closure audit preserved 1,346 uniquely named closed/manifold geometry definitions across 19 archived models. Another 42 baseline definitions have repeated names; every candidate mesh in each matching name group is closed and manifold. This confirms those groups conservatively without claiming unique occurrence matching. Two corpus models lacked usable baseline topology archives.

Validation: 222 kernel tests passed with one existing ignored test; all four independent STEP closure fixtures passed. The kernel regression also asserts that every shared source wire sample survives interior refinement.

Evidence: [Meanwell counts](meanwell.json), [pair comparisons](pair-summary.json), [closure audit](closure-summary.json), [repeated-name groups](repeated-name-validation.json). Raw CAD vertices and private diagnostics are excluded.

The other ten corpus models retain exactly the same verdicts, confirmed pair lists and unresolved pair lists as the axis baseline. Fresh single-run import times (seconds; not a performance benchmark):

| Printer | Axis baseline | Seam fix |
| --- | ---: | ---: |
| Voron V0 | 13.10 | 12.45 |
| Voron V2.4 | 41.07 | 40.85 |
| Switchwire | 7.29 | 7.46 |

[Other-model report equivalence and import times](other-model-status.json).

The initial released-axis gate used Look `510cef71a6e6a41b6f11ed003b587e2b121f4fec` and passed 77 Burr tests, strict Clippy and viewer proof. The final pin carries that fix onto the merged import kernel.

## Gate after the import integration merged

The final Look branch is `fix/periodic-seam-sampling-039`, head `01dbeac7c63e523019c64e0174cc2729ef991220`, based on merged kernel `58c39f4ba9ebabd28832e5c55193f85d9cc5286d`. Its algorithm revision is 10. This preserves the combined assembly-import and signed-torus fixes.

Fresh comparisons retain all 21 corpus verdicts and confirmed/unresolved pair lists from the merged import kernel. The OCCT gate retains 97 true pairs and zero false pairs, including protected 64/0, Bowden 9/0, Base's 16 and Center's eight. Bowden retains the same limited reference scope described above. Models whose reference or geometry matching remains incomplete are not counted as complete collision scans.

A fresh baseline/candidate closure audit covers all 21 models: 1,704 uniquely matched closed/manifold geometry definitions are preserved, plus 42 baseline definitions in entirely closed matching-name groups. There are no closure regressions or unproved groups in this audit. The Meanwell change remains 0 boundary / 1 non-manifold edge to 0 / 0 on this combined base, with 256,332 to 256,336 triangles. The synthetic regression, all 222 kernel tests and all four STEP fixtures also pass on the combined base.

Evidence: [combined verdict and pair gate](combined-pair-status.json), [fresh combined closure gate](combined-closure-summary.json). The earlier files retain the separate released-axis-baseline measurements.

The final combined published pin passes full local `npm run check`: 79 Burr tests, strict Clippy and viewer proof.
