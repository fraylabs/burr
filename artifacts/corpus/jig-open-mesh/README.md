# Closed conical charts in the OpenArm jig

The original OpenArm jig contains one 36-face definition used by two occurrences. Its two narrow conical charts left 22 mesh boundary edges, making the pair check incomplete. The repair uses the accurate inner inverse first and a certified analytic cone chart only where needed; the final mesh is closed and the one pair check completes with no interference. Physical source boundary contributors are retained.

Source credit: Enactic OpenArm hardware 2.0.0, CERN-OHL-S-2.0. This directory contains authored implementation evidence and sanitized metrics; no source CAD, coordinates or screenshots.

Validation against shipped 0.40.1 retains all 1,814 previously closed meshes and all 97 known positives across 21 models. Lower TradRack becomes complete with five strictly source-validated additional positives, yielding 102 known matches. Twenty reports are unchanged. This is not an exhaustive separation or zero-false certificate. Bowden remains scoped to 18 historically reported pairs, with nine known positives and nine historically disputed pairs unresolved; unresolved-pair completeness is not claimed.

All 25 held-out imports retain current 0.40.1 source-face attribution, including conversion refusals, and recorded historical 0.39.0/0.40.0 lost-face counts. The V2.4 spike retains all 114 canonical boundary points, its four unrecovered source faces have unchanged terminal signatures, and both cable bridges remain closed. Look 199 library tests, four STEP fixtures, kernel 224 tests (one existing ignored), two inverse-retry contracts and full local and published-Git npm90/Clippy/viewer checks pass. The Owner independently reviewed real workbench solid and X-ray renders of the jig, lower TradRack and V2.4, plus the pulley workbench and its unchanged check results. Seven private screenshots and the Owner's observed check results support that review.

The isolated import gate uses ABBA/BAAB/ABBA blocks and six fresh samples per binary/model. It fails a median over 10% slower or a candidate entirely above the baseline range in every block. Individual samples and all scoped receipts are retained below; these are not general speedup claims.

| Import | 0.40.1 median | Candidate median | Gate |
| --- | ---: | ---: | --- |
| V0 | 15.774 s | 15.121 s | Pass |
| V2.4 | 45.223 s | 43.669 s | Pass |
| Switchwire | 9.681 s | 9.673 s | Pass |
