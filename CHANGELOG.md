# Changelog

## Unreleased

- Added `blind_pocket_back_wall_thickness`, which checks remaining host
  material behind declared blind pockets from `bottom_center_mm` to the host
  part `bbox_mm`.
- Bumped the default `actuator_mount` rulepack to `0.12.0`; mechanical M3
  heat-set insert pockets selected by that rulepack now need at least 2 mm of
  material behind the blind pocket bottom.
- Added good/bad build123d and gallery proofs for an insert pocket that has
  valid STEP blind-pocket geometry but fails because the back wall is too thin.

## 0.25.0

- Added default actuator rulepack coverage for counterbore edge material:
  mechanical counterbores now check the larger `counterbore_diameter_mm`
  envelope against free edges instead of trusting only the through-hole.
- Bumped the default `actuator_mount` rulepack to `0.11.0`; mechanical
  counterbores selected by that rulepack now need at least 3 mm of material
  around the head recess.
- Added good/bad build123d and gallery proofs for a counterbore that is safe
  when inset and fails when the head recess is too close to a free edge.

## 0.24.0

- Added `feature_edge_distance`, which checks a declared feature envelope
  against its host part bounding box instead of only checking circular hole
  center distance.
- Bumped the default `actuator_mount` rulepack to `0.10.0`; mechanical straight
  slots selected by that rulepack now need at least 3 mm of edge material around
  the full slot envelope.
- Added good/bad build123d and gallery proofs for a mechanical straight slot
  that is safe when inset and fails when placed too close to a free edge.

## 0.23.0

- Added `standoff_boss_support_link`, which checks that declared M3 standoff
  bosses name the hole or insert they support and align to its centerline, axis,
  and support diameter.
- Bumped the default `actuator_mount` rulepack to `0.9.0`; mechanical
  `standoff_boss` features selected by that rulepack now need
  `supports_feature_id`.
- Added boss-supported M3 mount gallery proof cases, including a passing boss
  support example, a thin boss support-wall failure, and a missing standoff boss
  STEP-presence failure.

## 0.22.0

- Added a spacing-envelope agent repair proof that turns a thin relief-ligament
  failure into exact source guidance, applies it through the generic repair
  runner, regenerates CAD, and verifies the repaired receipt passes.
- Added a versioned static docs bundle with a source manifest, generated
  release manifest, and local artifact check for fray-site consumption.

## 0.21.0

- Added `feature_pair_spacing`, a declared-feature ligament rule that checks
  the minimum spacing implied by selected holes, straight slots, or explicit
  circle/capsule spacing-envelope metadata.
- Added `burr-build123d.spacing_envelope(...)` for declaring custom feature
  spacing envelopes while keeping geometry creation in normal CAD code.
- Added printed-plate fixtures proving dense cosmetic relief features pass when
  declared spacing is wide enough and fail when declared features are too close.
  This is a design-rule check, not CAD constraint solving or FEA.

## 0.20.0

- Added `fastener_support_wall_thickness`, which checks declared boss/support
  diameter around M3 clearance holes and heat-set insert pockets.
- Added `standoff_boss` STEP-presence checking, proving declared raised bosses
  exist as matching boss cylinders and top faces in exported STEP files.
- Added `burr-build123d.standoff_boss(...)` and support-diameter metadata for
  boss-supported fasteners.
- Added bad/good build123d proofs for fastener boss wall thickness and standoff
  boss STEP presence.

## 0.19.0

- Added `burr explain --json`, which emits `burr.repair-packet.v1` JSON for
  receipt-backed agent loops.
- Repair packets from plain receipts rank failures and name fixes without
  inventing exact source edits.
- Repair packets from Burr repair reports preserve exact `source_hint`
  `before_text`/`after_text` repair actions.
- Added a multi-fixture source-hint repair proof for printed-plate and captured
  slider fixtures.

## 0.18.0

- Added Source Hint Contract V3 fields to repair-action `source_hint` objects:
  `edit_kind`, `selector`, `before_text`, and `after_text`.
- Added exact source-text validation for repair reports, proving each hint maps
  once to the before/fixed Python source and still matches design-data
  before/after values.
- Added a supplemental envelope repair action for actuator housing width when
  the fixed receipt requires both moved mount holes and a larger part envelope.

## 0.17.0

- Added required repair-action `source_hint` fields with source file path,
  feature id, editable value path, before/after design-data values,
  `exact_from_design_data` confidence, and rationale.
- Added `npm run check:repair-action-source-loop`, proving repair actions can
  edit a copied bad actuator CAD source and rerun Burr to a passing receipt.
- Extended repair-report validation to prove every `source_hint` maps back to
  exact before/after design-data values.

## 0.16.0

- Added `repair_actions[]` to Burr repair report JSON.
- Repair actions now suggest receipt-derived edge-distance deltas for actuator
  edge-distance failures and tie each suggestion to the fixed passing receipt
  and design feature. Burr still does not auto-edit CAD.
- Extended repair-report validation to prove actions map to failures and the
  fixed receipt verifies positive after margins.

## 0.15.0

- Added repair report artifacts to the Burr gallery release bundle.
- Added a receipt-backed actuator repair report in JSON and Markdown, linking
  the bad actuator receipt, measured failures, first fix, and fixed passing
  receipt.
- Added `npm run check:repair-report` to prove the portable report exists and
  contains the before/after repair facts agents and websites need.

## 0.14.0

- Added the before/after actuator repair proof narrative: bad CAD fails a Burr
  check, `burr explain` gives the fix order, and the fixed CAD passes.
- Clarified that the gallery repair story is receipt-backed proof. Preview
  images show the part, but Burr receipts prove the bad and fixed states.
- Added contract copy for rendering actuator repair cards as one loop instead
  of unrelated good and bad examples.

## 0.13.2

- Added triaged `burr explain` output so multi-failure receipts are sorted by fix order: stale artifacts first, missing declared STEP geometry second, unsafe dimensions third, and declared measurement issues after that.
- Added explain proof coverage for messy receipts with failures deliberately emitted out of order.

## 0.13.1

- Added a Burr-owned fresh-install release gate that installs the published CLI,
  initializes the starter build123d project, proves the starter passes, mutates
  the starter into an edge-distance failure, verifies `burr explain` reports the
  measured problem, restores the starter, and verifies it passes again.
- Added the fresh-install release gate to CI so the public package path is
  checked independently from local workspace examples.

## 0.13.0

- Added manifest-declared rulepack paths so a design can select a non-default
  rulepack without requiring CLI flags.
- Added `feature_count` and `numeric_range` rule kinds for breadth checks on
  dense plates, captured sliders, and other measurement-heavy CAD artifacts.
- Added printed-plate and captured-slider rulepacks plus a T-slot linear slider
  gallery example.
- Added burr-build123d helper methods for `rulepack`, `measurement`,
  `measurements_update`, and generic `feature` metadata.
- Added CLI negative fixtures for captured-slider clearance and capture-lip
  failures.

## 0.12.0

- Added a dense random-hole gallery example that proves Burr checks declared
  mechanical intent while ignoring cosmetic and undeclared visual holes.

## 0.11.0

- Expanded the Burr gallery artifact into a good-vs-bad proof gallery.
- Added intentional failing gallery examples for missing bearing-seat shoulders,
  missing counterbore recesses, through-hole insert pockets, and disconnected
  slot geometry.
- Added manifest `expectation`, `group`, and `failed_rules` fields so websites
  can render caught mistakes as proof, not broken cards.

## 0.10.0

- Added printable gallery examples for a shaft-bearing bracket, slotted motor
  plate, and electronics standoff deck.
- Added `burr explain` for receipt-based feature/rule/problem/evidence/why/fix
  output.
- Added gallery preview rendering with ignored PNG proof artifacts under
  `artifacts/gallery-previews/`.
- Added versioned gallery artifact bundles for website/release consumption.
- Tightened npm package contents so generated receipts and previews do not ship
  accidentally.
