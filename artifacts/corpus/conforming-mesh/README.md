# Conforming meshes and conservative curved contact

Look `fix/conforming-mesh` at `15310227a83c52e1cb9f84f4b3a57675dd6c5818`, based on Look's `burr` branch. Burr starts from main after #41. These measurements use the original 0.36 corpus and `compare_pairs.py` with the complete OCCT references, unique placement matching, source identities and sampled surface checks.

Three formerly incomplete verdicts become conclusive and agree with OCCT: Fly fails, Apollo flat passes, and Apollo corner passes. All seven previously correct verdicts remain correct. Across these ten models, 65 confirmed pairs match OCCT and zero confirmed pairs contradict it. Six additional OCCT-positive cover pairs remain explicitly unresolved; this is not a claim that every collision is found.

| Model | Boundary / nonmanifold edges before → after | Burr before → after | OCCT | Confirmed true / false | OCCT positives unresolved |
| --- | --- | --- | --- | --- | --- |
| openAMR cover | 26 / 11 → 0 / 0 | fail → fail | fail (22 pairs) | 16 / 0 | 6 |
| Adafruit 1400 switch | 27 / 4595 → 0 / 4519 | fail → fail | fail (11 pairs) | 11 / 0 | 0 |
| Fly Gemini DIN mount | 174 / 73 → 0 / 0 | incomplete → fail | fail (8 pairs) | 8 / 0 | 0 |
| Apollo flat | 120 / 100 → 0 / 0 | incomplete → pass | pass | 0 / 0 | 0 |
| Apollo corner | 128 / 100 → 0 / 0 | incomplete → pass | pass | 0 / 0 | 0 |
| Faze4 | 10 / 235 → 0 / 235 | fail → fail | fail (9 pairs) | 9 / 0 | 0 |
| TradRack toolhead | 0 / 0 → 0 / 0 | fail → fail | fail (6 pairs) | 6 / 0 | 0 |
| Manta DIN mount | 0 / 0 → 0 / 0 | pass → pass | pass | 0 / 0 | 0 |
| Pick/place arm | 0 / 0 → 0 / 0 | fail → fail | fail (3 pairs) | 3 / 0 | 0 |
| Automation control unit | 0 / 0 → 0 / 0 | fail → fail | fail (12 pairs) | 12 / 0 | 0 |

The table counts welded edges across unique imported definitions. Compound PCB definitions contain touching source solids, so aggregate nonmanifold counts are not equivalent to defects in an individual solid. All 130 switch source shells, all six cover shells, all seven Fly shells and both shells in each Apollo model are closed and manifold after the change. `solid-counts.json` records before/after counts per source shell and its MANIFOLD_SOLID_BREP IDs, with no vertex data. `pair-summary.json` records the ten-model comparison and unresolved candidate counts.

## Causes and fixes

- Face-local subdivision inserted vertices that were absent from the adjacent face's edge. Look now retains the canonical shared edge sampling and adds cylinder support in the interior of rectangular parameter strips.
- Curve and vertex endpoints disagreed within already admitted BREP tolerance. Canonical endpoints now use the existing incidence tolerance, rather than widening the weld tolerance.
- Independent chord approximations crossed in narrow planar annuli. Look detects proper crossings in source planar boundaries and refines only the implicated shared edges, making both adjacent faces use the same refinement.
- Periodic seam and sphere pole handling introduced duplicate caps, displaced pole vertices or a whole periodic orbit in place of a narrow trim. The fix preserves source trim samples, exact pole locations and valid seam lifts. It does not manufacture a full-period boundary from an unsupported two-sample edge.

Closing these meshes also exposed false collision witnesses near curved contacts. Burr now treats witnesses within nominal mesh sampling distance of gentle nonplanar facets conservatively. It continues looking for a stronger witness and otherwise reports `below_tessellation_resolution`. This moves the four extra cover and four extra Fly pairs to unresolved. It also moves six real cover overlaps to unresolved; the heuristic is a conservative ambiguity marker, not certification of curvature or of an exact geometric error bound. Shallow planar overlaps retain their coordinate-accuracy path.

## Validation and remaining work

Look's focused library tests and STEP fixtures pass. Test-first regressions cover canonical straight edges, cylindrical seams, a filleted box, and a phase-offset thin annulus. Burr adds an independently generated, OCCT-valid ring/cylinder contact fixture with zero exact common volume; its former false finding is now unresolved. A deeper curved overlap remains confirmed, and the existing shallow planar overlap regression passes. Full `npm run check` passes (66 tests plus viewer checks).

MiniSB Bowden is not included in the correctness total: 33 source shells improve from 3021 boundary / 217 nonmanifold edges to 933 / 5, but invalid source cowlings and blower geometry remain. A pair-limited OCCT scan finished all 18 reported findings: nine are true and nine are false, with valid source and common BREP for every pair. See `bowden-reported-pairs.json`. The eight screw/nut and screw/carriage contacts and collet/retainer contact need additional investigation; Bowden is excluded from the correctness total. One valid source shell also still has 24 boundary edges. A fail alone is not evidence of pair correctness. The arcade spring and dome remain open and are reserved for a separate spline-axis/seam follow-up. The remaining large corpus assemblies also need final remeasurement. No claim is made that all 14 original incompletes are resolved.

To reproduce topology, copy `scripts/corpus-topology.rs` to `examples/corpus-topology.rs`, build that focused example, and run it on a corpus model. For per-shell diagnostics use `scripts/corpus-shell-topology.rs` on private Look shell dumps. For pair comparison use `scripts/scene-dump.rs`, `scripts/corpus-bench.rs`, and `scripts/compare_pairs.py`; third-party CAD and coordinate-rich logs remain private. All build, test and corpus runs on the shared Mac use `/tmp/burr-build.lock` and four Cargo build jobs.
