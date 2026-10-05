# Native spline inverses: Voron 2.4 spike

Opening Voron 2.4 produced a metre-long spike from **Toolhead Revo Voron Front**, so the camera fitted the spike as well as the printer. Source face 1740108 (ordinary B-spline surface 24572) admitted an exterior Newton root. That root realized the boundary point on the extrapolated carrier while its trim chart generated unrelated interior geometry.

The policy now tries two native-domain starting guesses and prefers a native root only when its physical residual meets the existing tolerance. The old exterior result remains available when no qualifying native root is found. A cached control-point hull rejects impossible native searches; rational hulls are used only with positive finite weights. Certified periodic surfaces retain their existing inverse path. Source boundary samples are unchanged. A replacement also requires a regular derivative at both roots, after scaling the derivatives by the native parameter spans. Near a collapsed edge, multiple UV representatives realize the same physical point; preserving the source representative prevents trim reordering and nearly degenerate triangles on the V2.4 cable bridges.

The independent cubic-surface regression failed before the fix. A companion regression retains an accurate exterior inverse when no native inverse exists. A third regression failed before the rank guard: a nearly collapsed synthetic edge switched to a different native root, despite representing the same physical point. It now retains its original trim representative at two physical scales. The final source face is valid in OCCT; its mesh escaped the BREP face bounds by **1,336.88 mm** before and **0 mm** after. Both versions have 114 boundary edges, with identical boundary coordinate sets. See [face evidence](face-boundary-summary.json).

Both real workbench Solid images were inspected: the released image has the spike and incorrect camera fit; the final render has no spike and restores the printer fit. Both reports remain incomplete with the same four missing faces and no confirmed pairs. [Workbench evidence](workbench-summary.json) records reports, browser errors and image hashes. Raw source CAD, coordinates and images remain in local evidence.

The exact Look pin in 0.37.0 lacked this metre-long spike, but its Toolhead mesh already exceeded the source solid bounds by 41.17 mm. That older overshoot was not traced to a face.

## Import performance

Paired import-only measurements run sequentially under the shared lock, in baseline/candidate/candidate/baseline order. Values are medians of two runs. Individual runs and source revisions are retained in [performance evidence](import-performance.json); these measurements do not establish a general speedup.

| Model | 0.39.1 import | Candidate import |
| --- | ---: | ---: |
| Voron V0 | 12.65 s | 12.56 s |
| Voron V2.4 | 40.58 s | 39.16 s |
| Switchwire | 7.37 s | 7.11 s |

## Validation

192 Look unit tests and four closed-shell fixtures pass. Both V2.4 cable bridges remain closed/manifold, and the Toolhead remains within the corrected bounds. The full local npm check passes 79 tests, strict Clippy and real viewer proof. Paired import timings show no observed regression, and both final workbench images were inspected. The fresh 21-model audit retains all verdicts and confirmed/unresolved pair lists: 97 true pairs and zero false pairs, including protected 64/0, Bowden 9/0, Base 16/0 and Center 8/0. Bowden validates only the 18 previously reported pairs; full pair completeness remains unknown. All 1,750 previously closed geometry meshes are retained: 1,708 unique matches and 42 in all-closed repeated-name groups. See [pair/status evidence](pair-status-summary.json) and [closure evidence](closure-summary.json).
