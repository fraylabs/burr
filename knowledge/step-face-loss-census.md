# STEP face losses on the 0.40.3 baseline

The largest measured cause is a spherical seam that the tessellator maps to a
zero-area UV walk. A bounded investigation found no sound repair using the
existing incidence evidence. This change adds measurement tools, not a runtime
recovery or a claim that incomplete models now pass.

Baseline: Burr `9126918`, Look `387f94f030a286124057dd14036e85cbc0eecb47`,
release build on Boat. The older corpus analysis is not the current baseline.

| Target | Lost faces | Measured cause |
| --- | ---: | --- |
| Framework Laptop 13 CAD | 1 / 6652 | Extrusion `BoundaryProjectionFailed` / `EvaluatorOutOfDomain` |
| FWKNAQ9_G01 | 6 / 5752 | 2 planar `WireNotClosed`; 4 planar `RejectedDegenerate` |
| jubilee.STEP | 0 / 9453 | FAIL, with unresolved pairs; no `step_faces_lost` |
| 6x_well_plate_bed_assembly | 1 / 1198 | NURBS `ConstraintRoleMissing` |
| kicadStepUpMod demo | 1 / 401 | B-spline `ContradictoryDualParity` |
| OpenArm_2.0 | 12 / 7520 | 10 `EdgeTraversalUnresolved`; 2 NURBS `NoOddParityRegion` |
| OpenArm_Cell+OpenArm_2.0 | 2 / 69931 | NURBS `NoOddParityRegion` |
| Positron | 174 / 140054 | 163 spherical and 4 other `NoOddParityRegion`; 6 traversal failures; 1 constraint-insertion failure |

The seven models with lost faces remain INCOMPLETE; Jubilee remains FAIL with
15 findings and 1,441 unresolved pairs. No runtime recovery was accepted, so
there is no improved after result. Framework's face `#128324` uses extrusion
`#128317`, three B-spline edges and a line; projection succeeded for 22 of 65
samples and refused 43 as evaluator-out-of-domain.

[The retained summary](../artifacts/face-loss/baseline-summary.json) includes all
54 model results, finding pairs, cold timings, 197 target face records with
surface/edge types and typed refusal evidence, and the repeat comparison.

Full corpus2 measured 33 models: 1 PASS, 12 FAIL, 20 INCOMPLETE. The 21 original
corpus models were also measured. All 30 reports present in both runs retained
identical finding, contact and unresolved-pair sets and outcomes. This is repeat
baseline evidence, not a repair-versus-baseline experiment. Runtime code and
dependency pins have no diff; no new pair can be attributed to these offline tools.

Gate findings repeated as Bowden 9, Base 16, Center 8, gearmotor 4. Their existing
unresolved counts are 76, 26, 8 and 2. The remaining 18 original models have 69
findings on this baseline, so the older protected count of 64 was not reproduced.
Unresolved pairs are not false pairs; no new OCCT classification was performed.

Cold 0.40.3 check seconds in the complete batch, in table order: 5.063, 0.748,
96.402, 1.336, 0.143, 2.104, 7.484, 35.767. The two jobs used fresh processes
and caches and serialized checks. Hardware fingerprints were not captured;
the retained repeat timings must not be interpreted as a speed regression or
improvement. There is no changed-runtime timing sample.

## Why a seam heuristic was rejected

162 Positron losses have one bound containing the same circle edge twice, with
opposite orientations. Example face `#2159925` references sphere `#753` and
edge `#1217057` through uses `#1693495` and `#1693496`. Both uses lift to
longitude pi, parity cancels, and the CDT has no triangles. Denser sampling
cannot distinguish the two uses of the same geometric arc.

On an exact sphere, an embedded open arc used on both sides is an artificial
cut: its complement is connected. Recovering the whole sphere would require
certifying that source arc's incidence and embedding on the carrier, retaining
its topology and face orientation. The example declares sphere radius `0.156`
and circle radius `0.156000000000001`. The representation uncertainty `0.01`
establishes neither exact equality nor that topological certificate. Existing
endpoint-incidence checks cannot certify the entire seam. Removing the bound,
widening closure tolerance, or choosing a UV material region would assert
geometry the current proof machinery has not established. These faces stay lost.

The remaining spherical loss has three edge uses and is outside this pattern.
The spline failures need separate source traversal, constraint provenance, or
parity proofs; they are not fixed by spherical seam handling.

## Reproduce and interpret the measurements

Build on Boat, including the ignored corpus fixtures explicitly:

```sh
cargo build --release --locked --example face_loss_census --bin burr
python3 scripts/face-loss-measure.py
python3 scripts/face-loss-measure.py --all-corpus2 --skip-census --phase before-full
python3 scripts/face-loss-causes.py artifacts/face-loss/before-full
python3 scripts/face-loss-compare.py artifacts/face-loss/before artifacts/face-loss/before-full
```

The driver uses fresh processes and caches, serial HTTP checks, full pair reports,
timings, and model stderr. Conversion losses come from the production shell API;
structured stderr diagnoses later tessellation losses. The census also records
strict public edge conversion errors. Those are labelled separately: production
uses deferred incidence, so an extra strict error is not a production loss.

HTTP errors leave evidence and make the batch exit nonzero. The corrected driver
encodes spaces as `%20`, matching the server's percent decoder. Python is offline
measurement tooling only; Burr's runtime remains pure Rust.
