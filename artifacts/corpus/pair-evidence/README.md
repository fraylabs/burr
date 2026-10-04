# Pair-level interference comparison

Burr 0.36.0's correct overall verdicts concealed pair errors. The robot cover's 17 findings contained 12 real pairs and five false pairs; ten real pairs were missing. The switch's four findings were all false pairs; eleven real pairs were missing.

This Burr-side change confirms 52 OCCT-positive pairs across the seven requested completed checks, with zero confirmed false pairs. Eleven OCCT-positive pairs remain explicitly inconclusive because their tessellated components have open boundaries. This is not a claim that the pair lists are complete. `pair_set_complete` and `unresolved_pairs` distinguish overall failure from a complete collision list. The Look tessellation repair is a separate integration owned by Burr Owner.

| Model | OCCT positive pairs | 0.36 true / false findings | After true / false findings | OCCT-positive pairs still inconclusive |
|---|---:|---:|---:|---:|
| openAMR robot cover | 22 | 12 / 5 | 12 / 0 | 10 |
| Adafruit 1400 switch | 11 | 0 / 4 | 10 / 0 | 1 |
| Faze4 | 9 | 9 / 0 | 9 / 0 | 0 |
| build123d pick/place arm | 3 | 3 / 0 | 3 / 0 | 0 |
| build123d control unit | 12 | 12 / 0 | 12 / 0 | 0 |
| Manta mount | 0 | 0 / 0 | 0 / 0 | 0 |
| TradRack toolhead | 6 | 6 / 0 | 6 / 0 | 0 |

## Causes and changes

- **Touching solids within one occurrence.** Distance deduplication merged coincident exit/entry faces, turning a ray crossing count from even to odd. This falsely put points outside the switch inside it. Interior classification now counts oriented crossings before distance deduplication. Consistently oriented closed boundaries use winding; closed meshes with inconsistent face winding retain a raw-crossing parity fallback. Ambiguous edge/vertex rays still recast. The synthetic touching-box regression reproduced the false interior result before the change.
- **Shared edges and straight T-junctions.** Requiring exactly two triangles per welded edge marked touching PCB solid compounds open. Directed boundaries now cancel, and unmatched edges are split for the closure check at existing collinear endpoints. A BVH limits the search. Positions and triangles stay unchanged; no holes are filled and weld tolerance is unchanged. This recovers the switch's Board and Copper meshes and ten real switch pairs. A synthetic T-junction failed before the change; removing a real face remains inconclusive.
- **Placement precision.** Mesh-local epsilon omitted the error in `f32` occurrence transforms. Casting those matrices to `f64` did not recover the missing precision. The five cover false witnesses were outside one exact OCCT solid. The coordinate budget now includes local coordinates, transform scale, translation magnitude and cancellation. Sub-resolution evidence is retained as `below_coordinate_resolution`, with its witness and budget. The far-from-origin shallow-overlap regression failed before the change; the existing resolvable shallow-overlap regression still passes.
- **A short sampled crossing.** After the PCB closure repair, top copper against the switch produced a 0.0004952 mm crossing, while OCCT Common volume was zero. Edge probes now continue past intervals below a nominal sampling floor derived from the two definition diagonals and Look's public relative-deflection policy. If no stronger vertex/interior/crossing witness exists, the pair is `below_tessellation_resolution`, with the witness and sampling floor. This is a conservative probe criterion, not an exact BREP error certificate; Look does not expose per-face analytic error bounds. The thin-bar synthetic uses corner vertices so it requires an edge crossing; authentic interior vertex witnesses and larger crossings retain priority.

Unresolved pairs are shown and can be highlighted in the Checks tab. A fail summary includes the unresolved count. Inconclusive candidates are not counted as interfering pairs; many are actual contacts or bounds candidates with zero exact overlap.

## Remaining exact pairs

Indices below are **OCCT occurrence indices**, connected to Burr indices by each JSON file's placement/identity mapping. They are not matched by name order.

- Cover nut/screw pairs: `[3,4]`, `[20,24]`, `[21,25]`, `[22,26]`, `[23,27]`, `[28,29]`, `[45,49]`, `[46,50]`, `[47,51]`, `[48,52]`. Each exact Common volume is approximately 4.762140007 mm³. The shared screw definition has 26 boundary edges and 11 nonmanifold edges. Its source BREP is OCCT-valid and no declared faces were lost, but the tessellation is not watertight. Every pair appears in `unresolved_pairs` as `open_component_mesh`; these are not shallow overlaps.
- Switch `[0,7]`: bottom copper against C0805 capacitor, exact Common volume 0.003859468662 mm³. The capacitor mesh has 27 boundary edges. Its source BREP is valid, but its mesh remains open. It is explicitly `open_component_mesh`.

There are no unexplained missing OCCT-positive pairs and no confirmed extra pairs in these seven models. There were no broad-phase placement mismatches for the missing pairs. The other fourteen corpus models have not been reclassified by this Burr-side run.

## Matching and reproduction

`compare_pairs.py` requires equal occurrence counts, a unique placement match, compatible source names where informative, and world-space surface samples agreeing with the exact placed OCCT geometry. It refuses bounding-box errors above 0.1 mm or an alternative placement margin below 0.2 mm. Those are **matching limits**, not interference tolerances. The smallest alternative margin in this corpus was about 0.469 mm. Names of repeated screws/resistors alone never determine the mapping. Flat exports use the same OCCT solid split as the baseline reference.

The JSON files retain the model SHA-256, both pair sets, the occurrence mapping with identity/placement/sample checks, and every remaining pair's explicit reason. No third-party STEP or mesh files are added. Source attribution/licences remain in [sources.csv](../sources.csv): openAMRobot/openamr-platform-hw, Adafruit/Adafruit_CAD_Parts, Faze4-Robotic-arm, the build123d assembly examples, VoronDesign/Voron-0 and Annex-Engineering/TradRack.

Use the original corpus's CadQuery 2.8.0 / OCCT 7.9.3.1.1 environment, with NumPy and SciPy. Copy `scripts/corpus-bench.rs` and `scripts/scene-dump.rs` into a temporary worktree's `examples/` directory, then build the two examples with the shared build lock, four Cargo jobs and `/tmp/burr-target`. Run one model per process under the same lock and capture its bench JSON and scene-dump JSON. Then:

```sh
python artifacts/corpus/scripts/compare_pairs.py MODEL.step \
  --scene MODEL.scene.json --burr MODEL.burr.json \
  --occt MODEL.occt.json --output MODEL.pair-comparison.json \
  --reference-complete
```

`--reference-complete` is required only for legacy OCCT logs without a completion flag, and asserts the full scan documented by the original corpus baseline. Partial reference scans are rejected. The OCCT positive-volume floor remains `max(1e-6 mm³, smaller-part volume × 1e-9)`.

Validation: all seven Burr checks rerun; both before/after pair comparisons validated independently against the placed OCCT occurrences; synthetic regressions for touching solids, T-junctions, real holes, placement precision and narrow crossings. Full local `npm run check` is required before pushing.
