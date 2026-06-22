# Changelog

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
