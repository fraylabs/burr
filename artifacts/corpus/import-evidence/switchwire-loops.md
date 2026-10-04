# Switchwire: malformed source bounds remain incomplete

The 42 `WireNotClosed` faces in the recorded Switchwire assembly all belong to
`CW2_Printed/Main_Body` (shell #652923, solid #21846, product definition #1353852).
They cannot be recovered by decomposing disconnected cycles while retaining
all the edges the source names. No importer or meshing behavior was changed
for these faces; all 42 remain refused.

| Source topology | Faces | Evidence | Outcome |
|---|---:|---|---|
| Open chains, including odd-degree vertices | 28 | Nearest unmatched endpoint gaps range from 0.2 to 11.53 mm; exact point IDs and exact coordinate aliases do not close them. | Incomplete: closure would require changed vertices or invented edges. |
| Balanced disconnected cycles | 14 | Every face contains a repeated edge component whose source vertices are 0.44–62.36 mm off the carrier; its other components lie on the carrier. Twelve have unique successors; two branch. | Incomplete: splitting preserves an invalid component; dropping it changes the source. |

For example, cylinder face #644488 has two radius-3.5 boundary circles on its
carrier. The packed loop additionally references circle edge #395922 twice in
opposite directions. That edge's source vertices lie 13.08 mm off the cylinder.
It cannot be interpreted as a seam of that face. The source declares a 0.01 mm
connectivity uncertainty. This is a foreign edge reference, not rounding at an
otherwise shared vertex. The exporter header identifies Autodesk Translation
Framework 12.6.0.85 / ST-DEVELOPER 19.2; the evidence does not establish why the
exporter produced these references or which replacement edges it intended.

Source-preserving single-face reductions were transferred through OCCT 7.9.3.
Forty-one transferred faces fail BRep validity checks. The remaining face
(#644662) reports zero area, while its source bound names off-carrier vertices;
validity alone does not establish preservation of source edge uses. OCCT's
transferred edge counts frequently differ from the source. None of these
transfers proves a safe reconstruction of the original bounds. The existing
whole-assembly OCCT pair reference is partial and reports invalid solids.

An authored synthetic annulus regression first failed with `WireNotClosed`.
Its packed eight-edge loop has an unambiguous outer square and hole, unlike
the real cases. Raw OCCT transfer reports area 96 but an invalid single wire.
The experiment was not enabled as a recovery rule: it does not justify the
foreign seams or open chains in the real model. Its fixture and failing log
remain local, alongside the source reductions; no third-party CAD is committed.

The summary contains source IDs, errors, and the source checksum. To regenerate
all 42 local reductions and the OCCT measurements, use the same OCCT reference
Python environment as the other corpus scripts, under the shared run lock:

```sh
until mkdir /tmp/burr-build.lock 2>/dev/null; do sleep 15; done
CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/burr-target \
  /tmp/burr-occt-venv/bin/python artifacts/corpus/scripts/inspect_step_loops.py \
  --model artifacts/corpus/models/Voron-Switchwire__Switchwire_Assembly_v1_STEP.step \
  --output /tmp/burr-import/evidence/switchwire-final
result=$?
rmdir /tmp/burr-build.lock
exit "$result"
```

This inspects original vertex positions against each OCCT carrier, using
standard OCCT STEP transfer defaults. The script performs no additional wire
repair, snapping, or edge substitution. Reductions retain the source header,
units, context and entity IDs. Keep the generated STEP files local. Source
licence and attribution are recorded in `artifacts/corpus/sources.csv`.

## Remaining chart refusals

The initial 66 chart/constraint/degenerate refusals remain ranked below. This
import work does not alter the chart triangulator or claim these are repaired.

| Recorded reason | Faces | What remains unresolved |
|---|---:|---|
| ContradictoryDualParity | 33 | Conflicting material parity in the chart; source of the conflict needs proof. |
| ConstraintInsertionIncomplete | 12 | Not all required source constraints were inserted. |
| NoOddParityRegion | 11 | No material region was established from the constraints. |
| RejectedDegenerate | 9 | Degeneracy was rejected; a safe region was not established. |
| ConstraintRoleMissing | 1 | A required constraint lacks a proved role. |

After torus recovery, six such refusals remain in the full robot and six in
Wheel. A full-robot example (#195824) has a local span around 1.25 mm but a
global-model chord tolerance around 2.25 mm. This is a candidate for chart
resolution investigation, not evidence that a smaller tolerance fixes the
material region. Meshing changes are being handled separately; import work
keeps these faces incomplete.
