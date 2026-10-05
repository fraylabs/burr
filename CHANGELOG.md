# Changelog

## Unreleased

## 0.40.1 - 2026-10-05

- Fix a 0.40.0 spline-meshing regression: Jubilee's pulley assembly retains
  all 510 source faces and the full Jubilee retains all 9,453 faces. The
  pulley bracket's interference with the M4 shoulder screw is reported again.
- Retry only a refused spline face's inverse search, preserving healthy
  meshes, the repaired Voron 2.4 toolhead spike and the closed cable bridges.
- The pulley's three M5 screw/t-nut pairs are real overlaps that OpenCascade
  Common misses. They remain explicitly unresolved as open meshes; they are
  not reported as contact or separated.

## 0.40.0 - 2026-10-05

- Fix the false interference reported in 0.39.1 on the FreeCAD-library yellow
  gearmotor: two curved parts that only touch were counted as overlapping.
  Flat triangles next to curved trims now include the curved trim's chord
  error in the penetration budget. The gearmotor now reports exactly its four
  real overlaps; the two-part contact case is reported as unresolved instead
  of failing.
- Checks panel: real interference findings are listed first; unresolved pairs
  are grouped by reason and part (for example "Board x LTV-817S - 9 pairs"),
  with one click to highlight every occurrence in a group.
- Burr now proves non-interference for many face-to-face pairs from the exact
  STEP planar and cylindrical surfaces, within the file's declared precision.
  These are listed as "contact or separated" rather than unresolved: on the
  Faze4 robot arm, 126 unresolved pairs become 93 contact-or-separated and 33
  unresolved. All 100 such pairs in the corpus have exactly zero common volume
  in OpenCascade. Burr does not claim the parts touch, only that they do not
  overlap.
- Generic solid names ("SOLID", "Body") fall back to the nearest named product,
  numbered to tell siblings apart.
- Fix a mesh defect that sent one B-spline face of the Voron 2.4 toolhead about
  1.3 m out of its part, which broke the default camera view.
- Results on the 21-model corpus are unchanged: 97 real interfering pairs
  found, 0 false. Import times are unchanged within measurement noise.

## 0.39.1 - 2026-10-05

- Close periodic NURBS seams whose period is only known from the source
  surface: the Meanwell power supply in the Voron 2.4 assembly is now a closed,
  manifold mesh again (the known limitation from 0.38.3 is fixed).
- No interference result changes: all 21 corpus models keep identical
  verdicts and pair lists (97 real pairs, 0 false), and every previously
  closed mesh stays closed.

## 0.39.0 - 2026-10-05

- Read STEP assemblies whose shapes use `MANIFOLD_SURFACE_SHAPE_REPRESENTATION`:
  the full openAMR robot platform now loads all 688 components instead of one
  flattened body.
- Read toroidal faces with a negative major radius (the spindle convention
  OpenCascade accepts) instead of refusing them.
- More conclusive results on real models, still with no false pairs. On the
  21-model corpus checked pair by pair against OpenCascade: 97 real
  interference pairs found and 0 false. The openAMR base now reports 16 real
  overlaps and the center bracket 8; both were previously incomplete.
- Import times are unchanged or slightly faster (Voron V0 13.5 s, Voron 2.4
  37 s, Switchwire 7 s on the test machine).
- Incomplete imports and unresolved pairs keep explicit reasons.

## 0.38.3 - 2026-10-05

- Fix a load-time regression from 0.38.0: STEP spline faces stored reversed
  could be meshed with the wrong closure axes, asking for grids of over
  100,000 points. The Voron Switchwire assembly took more than two minutes to
  import on 0.38.0 to 0.38.2; it loads in about 7 seconds again (6.8 s on
  0.37.0). Voron V0 and 2.4 load times are unchanged or slightly faster.
- The same fix halves the open edges on the Adafruit arcade button dome.
- Interference results are unchanged: 64 real pairs and 0 false on the
  benchmark models, 9 real and 0 false on the Bowden mount.
- Known limitation: one face of the Meanwell power supply in the Voron 2.4
  assembly now has one non-manifold seam edge. That model's check result is
  identical to 0.38.2 (incomplete because of four lost source faces).
- Add the fold-flat hanger example with a `burr.project.v2` hinge-motion
  configuration.

## 0.38.2 - 2026-10-05

- Large assemblies now open in the browser. The viewer no longer embeds every
  part's geometry in one huge HTML page: each part definition's mesh is stored
  once as a compact binary file, fetched by the page and drawn as instances for
  every occurrence, and selection highlights change instance colours instead of
  copying geometry.
- On the Voron V0 (1,691 parts) and Voron 2.4 (1,428 parts) assemblies, 0.38.1
  showed no frame within two minutes in Chrome. 0.38.2 shows the first complete
  frame after 20 s and 52 s including import, with the page down from about
  2 GB to under 1 MB and peak memory down from 7.8 GB to 4.1 GB and from 12.3 GB
  to 6.3 GB. The viewer now adds almost nothing on top of loading the model.
- Interference results are unchanged.

## 0.38.1 - 2026-10-05

- Unresolved pairs now carry the right reason: a planar overlap that is only
  found at coordinate resolution is no longer labelled as below tessellation
  resolution.
- Nearest-surface lookups compare true distances and treat ties
  consistently, so results no longer depend on mesh traversal order.
- Corpus results are unchanged from 0.38.0: 64 real pairs and 0 false on the
  benchmark models, 9 real and 0 false on the Bowden mount.

## 0.38.0 - 2026-10-05

- Tessellate STEP solids into closed meshes: neighbouring faces now share the
  same points along their common edges, cone bands are no longer meshed twice,
  and sphere poles and cylinder seams close cleanly. On the 21-model corpus
  this closes the openAMR cover screw, the Adafruit switch capacitor, the
  Voron Fly mount and both Apollo enclosures.
- More conclusive results, still with no false pairs. Checked pair by pair
  against OpenCascade on every corpus model whose result changed: 64 real
  pairs found and 0 false across the benchmark models, both Apollo enclosures
  now pass, and the MiniSB Bowden mount reports its 9 real overlaps.
- Contacts between curved parts that the mesh cannot resolve (screws seated
  in bores, rails and inserts) are listed as unresolved with a reason instead
  of as interference. This affects 9 pairs on the Bowden mount, 1 on the Fly
  mount and 6 shallow overlaps on the openAMR cover.

## 0.37.0 - 2026-10-04

- Interference pairs are now checked pair by pair against an exact OpenCascade
  reference on every corpus model whose check completes: 52 real pairs found,
  0 false pairs (0.36.0 listed 9 false pairs on two models).
- Pairs Burr cannot prove from the tessellation, because a part's mesh is open,
  are listed as unresolved with the reason, instead of being dropped or shown
  as interference. On the corpus these are 10 nut/screw pairs on the openAMR
  cover and 1 capacitor pair on an Adafruit switch; tessellation fixes for
  both are in progress.
- Placement rounding is bounded before a contact counts as overlap, which
  removes sub-tolerance witnesses on touching parts.

## 0.36.0 - 2026-10-04

Tested against 21 real public STEP assemblies (Voron printers, an open-source
robot, a robot arm, Adafruit parts and others) and an exact OCCT reference.
Burr now agrees with OCCT on 7 of them (up from 4) and says "incomplete" with a
reason on the rest, instead of crashing, timing out or guessing. No conclusive
result disagreed with OCCT.

- Load assemblies as assemblies: nullable STEP metadata (`$`) no longer drops
  product and placement records, occurrence placements are composed correctly,
  and flat multi-solid exports are split into parts.
- Recover faces that used to disappear (16,251 lost faces down to 134 across
  the corpus): fixed hyperbola parameter selection, reversed surface curves and
  collapsed line edges; a bad face is refused on its own instead of crashing
  the whole load.
- Report interference only for real shared interior volume. Touching parts,
  coincident sheets and zero-volume meshes no longer count; identical parts at
  the same placement still do.
- Check large assemblies in milliseconds: per-part BVHs, a broad-phase sweep
  and early structural checks replace whole-scene vertex welding.
- Say why a check is incomplete: lost assembly structure or lost faces are
  reported, with counts, before any pair is checked.
- Burr now builds against the `burr` branch of
  [fraylabs/look](https://github.com/fraylabs/look), which carries these
  geometry-kernel fixes.

## 0.35.1 - 2026-10-04

- Publish prebuilt binaries for macOS (Apple Silicon and Intel) and Linux
  x86-64 with every release, plus a checksum-verifying `install.sh`, so Burr
  installs in seconds without a Rust toolchain.

## 0.35.0 - 2026-09-26

- Update Look and its vendored geometry kernel; x86-64 builds now require AVX
  and FMA.

## 0.34.0 - 2026-09-03

- Replace two-STEP motion endpoints with one STEP assembly plus explicit
  revolute and prismatic joints, so Burr tessellates each motion model once and
  follows its declared hinge arcs and linear travel in the browser.
- Introduce `burr.project.v2` for the single-source motion contract, with
  fail-closed validation for model scope, component assignments, axes, pivots,
  angles, and travel.
- Keep source-pose interference results hidden while motion is playing or
  paused elsewhere, so an unchecked animated pose is never presented as clean.

## 0.33.0 - 2026-09-03

- Report real model-loading stages in the workbench while Look reads,
  tessellates, prepares materials, and builds the browser viewer.
- Reuse content-addressed viewer HTML across Burr processes, with bounded
  platform cache storage and owner-only permissions on Unix systems.
- Prioritize making the model visible before running assembly interference;
  checks now begin when the Checks tab is opened.
- Add automated memory/disk-cache and source-invalidation proofs plus a release
  measurement harness for the published hanger and digital-photo-frame outcome
  packs.

## 0.32.0 - 2026-09-03

- Exclude generated `__cadgen__` render caches from model discovery so Burr's
  sidebar does not expose temporary or incompatible GLB artifacts.
- Add one-click PNG snapshots of the current Look viewport, preserving the
  selected model, camera, theme, and X-ray or Solid presentation.
- Add named STEP assembly motions in `.burr/config.toml`, with play/pause and
  timeline controls for rigid components whose geometry is unchanged between
  the configured poses.

## 0.31.0 - 2026-08-24

- Add `burr <folder>` (normally `burr .`), a Look-powered local browser for
  recursively discovered STEP, STL, and GLB files with a collapsible folder
  tree and automatic active-model refresh.
- Keep the workbench geometry-first: `.burr/config.toml` scopes model folders
  only.
- Add one geometry-native STEP assembly interference check with explicit
  pass/fail/incomplete outcomes, separated-versus-touching-versus-intersecting
  fixtures, containment detection, file-version caching, and selectable
  two-component highlighting in the Look viewer.
- Add X-ray and Solid viewer modes, defaulting to X-ray so enclosed
  or occluded assembly components remain visible.
- Remove the retired receipt/rulepack product, including the `init`, `check`,
  `explain`, and `stamp` commands, Python adapters, schemas, examples, repair
  automation, and publishing lanes. Historical releases remain available from
  Git history and version tags.
- Make the npm manifest a private task runner; Burr's distribution surface is
  the versioned public Git source until Look's dependency stack can support a
  crates.io release.

## 0.30.0

- Added the `burr.receipt.v2` three-state trust contract: `pass` means the selected
  rulepack was compatible and evaluated checks passed with complete required
  mechanical coverage, `incomplete` means Burr could not establish that
  coverage, and `fail` means a checked claim or the rulepack contract is invalid.
  The schema-version bump makes the new `incomplete` state explicit to existing
  receipt consumers instead of extending the old two-state v1 contract in place.
- Required explicit rulepack selection instead of silently falling back to a
  default rulepack. Artifact/process incompatibility, zero applied-rule
  coverage, unchecked mechanical features, and insufficient candidates for a
  pair-spacing claim now produce `incomplete` receipts and a nonzero exit.
  Multi-target checks now preflight every input and stage every serialized
  receipt before replacing outputs, preventing configuration or staging errors
  from leaving partial proof artifacts and reporting any rare replacement-time
  partial state explicitly.
- Versioned structured explain output as `burr.repair-packet.v2` (and
  `burr.repair-packet-list.v2`) because incomplete outcomes add scope, warnings,
  and normalized incomplete reasons that v1 consumers cannot safely ignore.
- Tightened rulepack validation for duplicate ids, unsupported rule kinds,
  unknown selectors, missing rule bounds, and `process_kind` and `insert`
  selector contracts.
- Bumped the printed-plate, tool-access, and boss-support rulepacks to `0.2.0`
  and added STEP-presence rules for declared adjustable slots, service-screw
  holes, and standoff bosses, closing previously unchecked gallery features
  exposed by the stricter coverage contract.
- Made warnings and checked/unchecked feature coverage visible in CLI output so
  a successful command cannot hide skipped mechanical intent. Explicitly
  non-mechanical unchecked features remain allowed in a passing receipt, while
  unknown or misspelled intent values conservatively remain coverage-required.
- Preserved `@fraylabs/burr` as an intentional public open-source npm
  distribution surface for a future publish, pinned future npm publication to
  public access, and added a dry-run package check that includes the
  machine-readable contract schemas.
- Split CI into static, core, CAD-proof, and release-candidate lanes with locked
  dependency checks, strict Clippy, stale-run cancellation, and timeouts. The
  release-candidate lane installs the packaged Rust CLI and both locally built
  Python wheels before accepting the change.
- Split the Fray website contract into a PR-safe source check and a published
  release-asset check in the manual fresh-install lane, so a release candidate
  no longer depends on its own not-yet-created GitHub release.
- Added a CI parity test that compares every rule kind's Rust field allowlist
  with its closed JSON Schema branch, preventing either contract from drifting
  independently.

## 0.29.0

- Added Practical Mechanical Lint V1: five focused rulepacks for hardware fit
  windows, tool access, mount-pattern consistency, printable retention tabs, and
  boss support.
- Added ten gallery-visible good/bad build123d fixtures proving those mistakes
  are caught from declared intent metadata and receipts.
- Added `npm run check:practical-mechanical-lint` and wired it into the
  aggregate check so the practical fixture set stays release-gated.

## 0.28.0

- Added default actuator rulepack coverage for M3 standoff boss edge material
  and M3 heat-set insert pocket edge material, using explicit boss and pocket
  envelope fields instead of the default hole diameter fields.
- Bumped the default `actuator_mount` rulepack to `0.14.0`; selected M3
  standoff bosses and selected mechanical M3 insert pockets now need at least
  3 mm of edge material around the declared boss or pocket envelope.
- Added Mistake Library V1 gallery proofs for tight captured-slider clearance,
  missing and shallow capture lips, excessive cosmetic relief holes, thin
  hole-to-slot ligaments, insert pockets near a free edge, and standoff bosses
  near a free edge.

## 0.27.0

- Added default actuator rulepack coverage for loaded bearing-seat edge
  material, using `seat_diameter_mm` to check the bearing seat envelope against
  free edges.
- Bumped the default `actuator_mount` rulepack to `0.13.0`; loaded bearing
  seats selected by that rulepack now need at least 3 mm of material around the
  seat.
- Added good/bad build123d and gallery proofs for a 608 bearing seat that has
  valid STEP seat geometry but fails when the seat is too close to a free edge.

## 0.26.0

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
