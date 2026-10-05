# Retry a refused spline face without changing healthy meshes

The 0.40.0 local inverse search could miss a remote native knot cell on an ordinary spline. This caused the Jubilee pulley to lose one source face and the full Jubilee assembly to lose three newly failing faces. The first causal commit was Look `d5abb2a`, which restricted native search to two Newton starts.

After every existing route refuses a face with `ContradictoryDualParity`, a typed retry enables native presearch and source knot-cell starts. The retry uses the same physical edge samples and carrier, preserves the original normal hemisphere for remote roots, and keeps the original refusal if it cannot recover. Healthy faces use the original search unchanged.

The pulley retains all 510 declared faces and reports bracket/shoulder-screw pair `[18,22]` again. Full Jubilee retains all 9,453 declared faces; its whole-assembly interference results are outside this import-only check.

Three additional pulley screw/t-nut pairs `[8,17]`, `[9,19]`, `[13,20]` are real overlaps independently established by source interior classification and fine-mesh winding, although OpenCascade Common returns empty. This face-only repair keeps them explicitly unresolved as `open_component_mesh`; it never calls them contact or separated. The broader coherent shell search is separate work. See the [sanitized independent source receipt](source-overlap-summary.json). No reliable total overlap volume is claimed for those three pairs.

Validation retains all 97 known positive findings and all 1,814 previously closed geometries in the original 21-model corpus, with identical reports. All 25 held-out imports retain face counts and failed-source identities against both 0.39.0 and 0.40.0. The repaired V2.4 spike keeps all 114 canonical boundary points and remains inside its source bounds; both cable bridges remain closed.

Tests: 194 Look library tests, four STEP fixtures, 222 kernel tests (one existing ignored), two retry-contract tests and five focused inverse tests passed. Full local npm checks include 90 Burr tests, strict Clippy and viewer checks. Real workbench solid and X-ray screenshots for pulley, Jubilee and V2.4 were independently captured and confirmed. The exact published Look Git dependency passed full npm checks and a production build as Burr 0.40.1; see [Git validation](git-validation.json).

The scoped import timing gate rejects a candidate median more than 10% slower or a candidate slower than the baseline sample range in every balanced block. All 36 individual measurements are retained.

| Import | Baseline median | Candidate median | Change |
| --- | ---: | ---: | ---: |
| V0 | 15.547 s | 15.039 s | -3.3% |
| V2.4 | 48.071 s | 46.708 s | -2.8% |
| Switchwire | 9.388 s | 9.335 s | -0.6% |

No raw CAD, coordinates or screenshots are included. A zero-volume Common result or bounded negative sampling alone is not an exhaustive separation proof.

V2.4 retains the same four failed source face IDs as 0.40.0: 1814162, 1814164, 1831656 and 1831657, with identical terminal reasons. The pulley’s 12 contact-or-separated pairs are awaiting their independent source audit; release remains held for that result.
