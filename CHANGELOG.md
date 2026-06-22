# Changelog

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
