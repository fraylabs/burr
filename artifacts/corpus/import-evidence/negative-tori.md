# Signed-major STEP torus import

Look now reads the negative-major-radius convention used by SolidWorks and ProE: the geometric major radius is its absolute value, the original UV coordinates remain unchanged, and bound wires reverse while the face retains its declared `same_sense`. A signed-radius equation or reversing the carrier's UV chart gives different geometry or pcurves.

This follows OCCT 7.9.3's [StepToGeom torus conversion](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/StepToGeom/StepToGeom.cxx) and [StepToTopoDS face and wire orientation](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/StepToTopoDS/StepToTopoDS_TranslateFace.cxx). On a spindle torus, the folded inner sheet also needs the opposite radial section for inverse and nearest lookup. Its normal and normal derivatives include the sign of the radial Jacobian, matching `du × dv`.

Look branch `fix/negative-radius-torus`, head `5a838db65125830ccd472661507ac71b6dcc5cc5`, includes the manifold-surface representation fix from `e5066137f6d21d370e5c80af3a8e4e434fc78841`. The new synthetic regressions failed before their respective fixes. They cover both meridian sheets, native STEP pcurves, bound-wire reversal, and both declared face senses. The geometry property tests, normal finite differences, 11 real-face regressions and 59 focused STEP input tests pass. The input suite retains the existing exclusions for `table::read`, `assy::occt_assy`, and `tessellate_shape::tessellate_shape`.

All 26 negative-radius source faces were reduced locally, retaining their original geometry, header, units and entity IDs. OCCT and Look were compared face by face, with 529 interior samples on each face and samples along every boundary curve. Every face passes point, oriented-normal, inverse and nearest checks, with no missing lookups. All 98,974 refined triangles have the same winding as OCCT. At a 0.001 mm mesh setting, maximum sampled distance to the exact trimmed face is 0.001587 mm, and maximum area error is 0.02273%. Analytic point error is below 3e-14 mm, normal error below 9e-16, and boundary inverse error below 6e-10 mm. `torus-faces.json` records each source hash and face's results.

The source reductions, OCCT sample data, detailed meshes and logs remain local in `/tmp/burr-import/evidence/tori/`. No source CAD is committed. Source attribution is openAMRobot/openamr-platform-hw, CERN-OHL-P-2.0; see `../sources.csv`. The only new committed STEP fixture was written analytically by Fray Labs under MIT OR Apache-2.0.

To reproduce the face check, run these commands sequentially under the shared build lock with four Cargo jobs. Use the documented CadQuery/OCCT Python environment, a local evidence directory, and the original corpus:

```sh
python artifacts/corpus/scripts/validate_tori.py --corpus CORPUS_DIRECTORY --output LOCAL_TORI_DIRECTORY
# In the pinned Look checkout:
cargo build --release --locked --example torus_face_probe
# Copy the binary before releasing the shared build lock.
LOCAL_PROBE LOCAL_TORI_DIRECTORY/reference.json > LOCAL_TORI_DIRECTORY/look.json
# In the Burr checkout:
python artifacts/corpus/scripts/compare_tori.py LOCAL_TORI_DIRECTORY
```

The recorded probe and corpus binaries were built from the torus implementation at `0a89c3535f26cb51bc38c0db9faebc7cfc62030c`; the final Look head adds only a cache identity revision. Burr also includes the Look meshing policy identity in its viewer cache key, invalidating previously cached views after an import algorithm change.

| Model | Lost faces before | Lost faces after | Verdict before | Verdict after |
|---|---:|---:|---|---|
| Full robot | 18 | 6 | incomplete | incomplete |
| Base | 4 | 0 | incomplete | incomplete |
| Wheel | 14 | 6 | incomplete | incomplete |
| Center bracket | 2 | 0 | incomplete | fail |
| Other 17 models | baseline | unchanged | baseline | unchanged |

All 21 models and six repros were rerun. The seven existing full occurrence comparisons retain 52 true and zero false pairs. Center's eight newly confirmed pairs are all OCCT positives. Its nine participating occurrences pass the same 0.1 mm placement limit, 0.2 mm uniqueness margin, source identity and 0.1 mm surface-sample checks. Across those nine occurrences, maximum box error is 0.000024 mm and surface error is 0.000024 mm; the nearest alternative placement is over 41 mm away. The new `compare_pairs.py --reported-pairs-only` mode verifies every reported positive while leaving unrelated occurrences and unresolved candidates unverified. No matching or interference tolerance was relaxed. Eight additional OCCT-positive Center pairs remain unconfirmed.

Full occurrence matching still refuses Base (0.262 mm box error), Wheel (0.613 mm) and Center's unrelated occurrence 0 (0.303 mm). Base and Wheel report zero confirmed pairs and remain incomplete. The full robot also reports zero confirmed pairs and retains a partial OCCT reference scan. Across all 21 models, the only new confirmed pairs are Center's eight verified positives; none of the previous confirmed pair sets lost a pair. There are no new wrong conclusive verdicts. Incomplete corpus models fall from 14 to 13. The remaining six lost faces on both the full robot and Wheel have chart/constraint refusals.

The assembly comparison is recorded separately in `torus-summary.json`. Its baseline is the recorded 0.36 import census (the same Look revision as Burr 0.37) and the 0.37 pair summary. It is a freshly measured after run, not a freshly measured before run. Run `measure_imports.py` as documented in `README.md`, adding `--additional-pair-model` for the robot Base, Wheel and Center assemblies. Reproduce Center's positive-pair verification with the same scene, Burr report and completed OCCT reference using `compare_pairs.py --reference-complete --reported-pairs-only`. Unmatched occurrence identities remain explicitly unverified; a partial OCCT scan cannot certify a complete pair list.

Switchwire's disconnected loops and the 66 chart, constraint and degeneracy refusals remain separate work. This patch does not relax their refusal rules.
