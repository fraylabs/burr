# Burr

Burr is design-rule checking for CAD-as-code.

It gives agents and humans a hard feedback loop before a part becomes a print:

```txt
design file -> generated part -> burr-design-data.json -> Burr checks -> receipt
```

Burr does not design the part. It verifies declared mechanical intent against
metadata, dimensions, source/artifact freshness, and STEP geometry evidence.

## Why

CAD agents can make parts that look plausible while hiding bad edge distances,
missing holes, stale STEP exports, or decorative holes that should not be judged
as fastener interfaces. Image review helps, but it cannot reliably prove those
facts.

Burr turns CAD work into measurable receipts:

```txt
M3 loaded mounting hole
measured center-to-edge = 8.0 mm
required center-to-edge = 10.2 mm
result = fail by 2.2 mm
```

Then `burr explain` turns the receipt into fix guidance:

```txt
1. Fix dimension: move or resize unsafe geometry.
Feature: m3_lower_left
Category: unsafe dimension
Problem: the loaded M3 hole is too close to a free edge.
Why it matters: thin edge material can crack, delaminate, or fail.
Fix: move the hole inward or make the surrounding part larger.
```

When a receipt has multiple failures, `burr explain` sorts them by fix order:
stale artifacts first, missing declared STEP geometry second, unsafe dimensions
third, then declared measurement issues.

## Quickstart

Install from crates.io:

```bash
cargo install burr --version 0.27.0
```

Create and check a build123d starter part:

```bash
burr init my-part
cd my-part
uv run python design.py
burr check .
burr explain .
```

The generated starter installs `burr-build123d==0.10.0` from PyPI.

To prove the published install path from this repo:

```bash
npm run check:fresh-install
```

The fresh-install check also proves the starter failure-to-fix loop: move the M3
hole too close to the side edge, fail edge distance, explain the measured
problem, restore the hole, and pass again.

## Product Loop

Use Burr like tests for generated mechanical parts:

```txt
write or generate CAD
  -> emit burr-design-data.json with intended features
  -> export STEP
  -> burr check .
  -> burr explain .
  -> fix CAD or metadata
```

For an agent repair runner, keep the loop source-driven and receipt-backed:

```txt
generate CAD source and artifacts
  -> check: run burr check
  -> explain-json: run burr explain --json for a repair packet
  -> apply only exact source_hint before_text -> after_text edits
  -> regenerate design data and STEP from edited source
  -> check again
  -> stop when pass, or when no exact source edit is available
```

The packet is guidance, not an auto-editor. A plain failed receipt can rank the
problem and name the fix, but it cannot honestly invent exact source edits. An
agent should only apply a `source_hint` when `before_text` occurs exactly once
in the current source and the hint carries `confidence:
"exact_from_design_data"`. If the exact source text, selector, or design-data
value path does not match, or the packet has no exact `source_hint` edits left,
stop and ask for a new generation or human edit instead of guessing. The final
trust signal is the fresh regenerated passing Burr receipt, including source
and artifact freshness checks.

For Burr 0.14, the gallery explains the same loop as a before/after actuator
repair proof:

```txt
bad CAD -> Burr check -> explain fix order -> fixed CAD passes
```

Burr is not a constraint solver, FEA engine, slicer, or universal CAD brain.
It checks specific declared mechanical claims. A ligament rule only checks the
declared spacing between selected slots, holes, or cutouts; it does not prove
part strength or find every thin web in the CAD model. Workload/stress survival
belongs to later FEA/FEM or physical testing.

## Local Development

```bash
npm install
uv sync --all-packages
npm run check
```

Run the build123d adapter examples:

```bash
uv sync --all-packages
npm run check:build123d
```

Run the optional OpenCascade STEP backend proof:

```bash
uv sync --all-packages
npm run check:ocp
```

Run the mixed-intent CAD proof:

```bash
uv sync --all-packages
npm run check:mixed-intent
```

Run the counterbore CAD proof:

```bash
uv sync --all-packages
npm run check:counterbore
```

Run the counterbore edge-material proof:

```bash
uv sync --all-packages
npm run check:counterbore-edge-distance
```

Run the bearing-seat edge-material proof:

```bash
uv sync --all-packages
npm run check:bearing-seat-edge-distance
```

Run the fastener support wall proof:

```bash
uv sync --all-packages
npm run check:fastener-support
```

Run the standoff boss STEP-presence proof:

```bash
uv sync --all-packages
npm run check:standoff-boss
```

Run the standoff boss support-link proof:

```bash
uv sync --all-packages
npm run check:standoff-boss-support-link
```

Run the straight-slot CAD proof:

```bash
uv sync --all-packages
npm run check:slots
```

Run the generic feature edge-distance proof:

```bash
uv sync --all-packages
npm run check:feature-edge-distance
```

Run the printable example gallery:

```bash
uv sync --all-packages
npm run check:gallery
npm run gallery:render
npm run gallery:artifact
```

The build123d examples and gallery commit only source and docs. STEP files,
`burr-design-data.json`, receipts, and preview PNGs are generated by the example
scripts and ignored by git. Preview PNGs are visual review artifacts; Burr
receipts remain the verifier.

Boss meat around fasteners is checked from declared mechanical intent. A
boss-supported M3 hole or insert pocket should declare the inner hole/pocket
diameter and `support_diameter_mm`; Burr then checks the radial wall around the
fastener. This catches the common case where a rendered boss looks plausible but
has too little material around the screw or insert.

Boss existence is a separate STEP-presence claim. Declare a `standoff_boss`
feature for the raised support body; Burr checks that the exported STEP contains
the boss cylinder and top face. Together, the boss-presence and support-wall
rules prove both that the support is physically in the CAD and that its declared
radial material is large enough for the checked fastener.

Boss linkage is the third support claim. A mechanical `standoff_boss` should set
`supports_feature_id` to the clearance hole or heat-set insert pocket it
supports. Burr checks that the referenced feature exists and that the boss
centerline, axis, and declared support diameter align with that feature.

For website or release use, `npm run gallery:artifact` writes a versioned bundle:

```txt
artifacts/releases/burr-gallery-v<version>/
artifacts/releases/burr-gallery-v<version>.zip
```

The bundle contains PNG previews, passing Burr receipts, stamped design data,
and a manifest. Burr owns these generated proof artifacts; websites should
consume the zip or GitHub release asset read-only instead of regenerating CAD.
See [docs/fray-website-contract.md](docs/fray-website-contract.md) for the
website ingestion contract.

Static docs use the same release-artifact pattern:

```txt
npm run docs:artifact
npm run check:docs:artifact
artifacts/releases/burr-docs-v<version>/
artifacts/releases/burr-docs-v<version>.zip
```

The docs bundle contains Markdown docs plus package, rulepack, and license
references indexed by `manifest.json`. See
[docs/static-docs-bundle.md](docs/static-docs-bundle.md) for the fray-site
integration contract.

For the Burr 0.14 actuator repair proof, the gallery should read as one loop:
the bad actuator CAD fails with measured evidence, `burr explain` tells the
repair order, and the fixed actuator CAD passes. The preview is visual context;
the receipt is the verifier.

Start a build123d part:

```bash
burr init my-part
cd my-part
uv run python design.py
burr check .
```

## Commands

```bash
burr --version
burr init <folder>
burr check <folder|burr-design-data.json>...
burr explain <folder|burr-receipt.json>...
burr stamp <folder|burr-design-data.json>...
```

`init` creates a minimal build123d project with `design.py`, `pyproject.toml`,
and `.gitignore`. The generated project depends on `burr-build123d==0.10.0`
from PyPI.

`check` finds `burr-design-data.json`, runs freshness checks and rulepack
checks, then writes `burr-receipt.json` beside each design data file.

`explain` reads `burr-receipt.json` and expands failed checks into plain
feature/rule/problem/evidence/why/fix output.

`stamp` computes `sha256` and `size_bytes` for declared source and generated
artifact files.

## Build123d Helper

Burr does not replace build123d. The optional helper records design data while
your normal build123d file creates geometry.

```python
from build123d import Box, BuildPart, Locations, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole

design = BurrDesignData(
    artifact_id="my-actuator",
    artifact_type="actuator_mount",
    process={"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4},
)
design.source("design.py")
design.artifact("actuator.step")
design.part("housing", bbox_min=(-42, -16, 0), bbox_max=(42, 16, 26))

with BuildPart() as housing:
    with Locations((0, 0, 13)):
        Box(84, 32, 26)

    m3_clearance_hole(
        design,
        feature_id="m3_lower_left",
        part="housing",
        center=(39.5, -8, 8),
        axis=(1, 0, 0),
        role="loaded_mount",
    )

export_step(housing.part, "actuator.step")
design.write(DESIGN_DATA_FILE)
```

That one helper call cuts the hole in build123d and records the feature Burr
checks. Burr core still reads only `burr-design-data.json`, so other CAD tools
can use the same contract.

For custom rulepacks and non-standard features, the helper can emit plain Burr
metadata without a specialized geometry helper:

```python
design.rulepack("../../../rules/captured_slider.rulepack.json")
design.measurements_update({
    "head_side_clearance_mm": 0.25,
    "carriage_lip_each_side_mm": 3.5,
})
design.feature(
    feature_id="left_capture_lip",
    kind="capture_lip",
    part="carriage",
    role="lift_off_blocker",
    engagement_mm=3.5,
)
```

For custom reliefs, windows, and decorative cutouts, geometry still belongs in
normal CAD code. Add an explicit envelope only when a rulepack should check the
declared spacing around that feature:

```python
from burr_build123d import spacing_envelope

design.feature(
    feature_id="rounded_relief_window",
    kind="cutout",
    part="plate",
    intent="cosmetic",
    role="relief_slot",
    spacing_envelope=spacing_envelope(
        segment_start=(0, -8, 0),
        segment_end=(0, 8, 0),
        radius_mm=3.0,
    ),
)
```

For `feature_pair_spacing`, Burr uses an explicit `spacing_envelope` when one
is present. Otherwise it derives a circle from `center_mm` and `diameter_mm`, or
a straight-slot capsule from `center_mm`, `width_mm`, `length_mm`, and
`span_axis`. The receipt reports the closest declared pair, clearance, and
margin. It does not search the whole STEP for every thin region.

## Design Data

A lintable CAD artifact folder contains `burr-design-data.json`.

This file is the language-agnostic contract. It can be emitted by build123d,
CadQuery, OpenSCAD, JavaScript CAD, Rust CAD, Fusion scripts, or any tool that
can write JSON.

```json
{
  "schema_version": "burr.design-data.v1",
  "artifact_id": "linear-actuator-bad",
  "artifact_version": "0.1.0",
  "artifact_type": "actuator_mount",
  "units": "mm",
  "source": {
    "path": "source.py",
    "sha256": "..."
  },
  "artifacts": [
    {
      "kind": "step",
      "path": "actuator.step",
      "sha256": "..."
    }
  ],
  "parts": [
    {
      "id": "housing",
      "bbox_mm": {
        "min": [-42, -16, 0],
        "max": [42, 16, 26]
      }
    }
  ],
  "features": [
    {
      "id": "m3_lower_left",
      "part": "housing",
      "kind": "clearance_hole",
      "intent": "mechanical_interface",
      "fastener": "M3",
      "diameter_mm": 3.4,
      "center_mm": [39.5, -8, 8],
      "axis": [1, 0, 0],
      "role": "loaded_mount"
    }
  ]
}
```

### Declared Feature Intent

Burr does not infer that every cylinder or hole in a STEP file is mechanically
important. A STEP file may contain vents, lightening holes, fluid passages,
cosmetic cuts, construction reliefs, bosses, fillets, and unrelated round faces.

Burr judges only features that are declared in `burr-design-data.json` and
selected by the active rulepack. Use `intent` to separate mechanical interfaces
from incidental geometry:

```txt
mechanical_interface  -> judged by mechanical rulepacks
weight_reduction      -> declared if useful, but not judged by actuator rules
fluid_or_air_path     -> separate rules, not screw-mount rules
manufacturing_feature -> process-specific rules only
cosmetic              -> normally unjudged
```

For legacy design data, missing `intent` is treated as `mechanical_interface`.
Set `intent` explicitly when a declared feature should not be judged by
mechanical rulepacks.

## Rulepacks

The included actuator mount rulepack checks loaded M3 clearance-hole edge
distance, whole-slot edge material, counterbore head-recess edge material,
loaded bearing-seat edge material, blind insert-pocket back-wall material,
standoff boss edge material, heat-set insert pocket edge material,
minimum wall thickness around M3 clearance holes, whether declared M3 clearance
holes exist as matching cylindrical geometry in the exported STEP, and whether
declared straight slots, counterbores, heat-set insert pockets, and bearing
seats exist as matching STEP cylinder/plane evidence:

```json
{
  "schema_version": "burr.rulepack.v1",
  "id": "actuator_mount",
  "version": "0.14.0",
  "rules": [
    {
      "id": "m3_loaded_hole_edge_distance",
      "kind": "hole_edge_distance",
      "applies_to": {
        "kind": "clearance_hole",
        "fastener": "M3",
        "intent_any": ["mechanical_interface"],
        "role_any": ["loaded_mount", "mount", "housing_mount"]
      },
      "min_center_to_edge_diameter_multiple": 3.0
    },
    {
      "id": "m3_clearance_hole_wall_thickness",
      "kind": "minimum_wall_thickness",
      "applies_to": {
        "kind": "clearance_hole",
        "fastener": "M3",
        "intent_any": ["mechanical_interface"]
      },
      "min_wall_thickness_mm": 2.0
    },
    {
      "id": "counterbore_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "counterbore",
        "intent_any": ["mechanical_interface"]
      },
      "diameter_field": "counterbore_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "bearing_seat_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "bearing_seat",
        "intent_any": ["mechanical_interface"],
        "role_any": ["loaded_bearing_support", "shaft_support"]
      },
      "diameter_field": "seat_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "m3_standoff_boss_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "standoff_boss",
        "fastener": "M3",
        "intent_any": ["mechanical_interface"],
        "role_any": ["pcb_standoff", "fastener_support", "bossed_mount"]
      },
      "center_field": "boss_center_mm",
      "diameter_field": "boss_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "heat_set_insert_pocket_edge_distance",
      "kind": "feature_edge_distance",
      "applies_to": {
        "kind": "heat_set_insert_pocket",
        "insert": "M3x5.7",
        "intent_any": ["mechanical_interface"],
        "role_any": ["threaded_mount", "pcb_standoff", "bossed_insert", "fastener_support"]
      },
      "center_field": "pocket_center_mm",
      "diameter_field": "pocket_diameter_mm",
      "min_wall_to_edge_mm": 3.0
    },
    {
      "id": "m3_insert_pocket_back_wall_thickness",
      "kind": "blind_pocket_back_wall_thickness",
      "applies_to": {
        "kind": "heat_set_insert_pocket",
        "insert": "M3x5.7",
        "intent_any": ["mechanical_interface"],
        "role_any": ["threaded_mount", "pcb_standoff", "bossed_insert"]
      },
      "min_back_wall_thickness_mm": 2.0
    },
    {
      "id": "m3_clearance_hole_step_presence",
      "kind": "feature_presence",
      "applies_to": {
        "kind": "clearance_hole",
        "fastener": "M3",
        "intent_any": ["mechanical_interface"]
      },
      "artifact_kind": "step",
      "diameter_tolerance_mm": 0.05,
      "centerline_tolerance_mm": 0.25,
      "axis_dot_min": 0.99
    },
    {
      "id": "straight_slot_step_presence",
      "kind": "feature_presence",
      "applies_to": {
        "kind": "straight_slot",
        "intent_any": ["mechanical_interface"]
      },
      "artifact_kind": "step",
      "width_tolerance_mm": 0.05,
      "endpoint_tolerance_mm": 0.25,
      "side_plane_tolerance_mm": 0.25,
      "axis_dot_min": 0.99
    },
    {
      "id": "counterbore_step_presence",
      "kind": "feature_presence",
      "applies_to": {
        "kind": "counterbore",
        "intent_any": ["mechanical_interface"]
      },
      "artifact_kind": "step",
      "bore_diameter_tolerance_mm": 0.05,
      "counterbore_diameter_tolerance_mm": 0.05,
      "centerline_tolerance_mm": 0.25,
      "counterbore_center_tolerance_mm": 0.5,
      "shoulder_plane_tolerance_mm": 0.25,
      "axis_dot_min": 0.99
    },
    {
      "id": "heat_set_insert_pocket_step_presence",
      "kind": "feature_presence",
      "applies_to": {
        "kind": "heat_set_insert_pocket",
        "intent_any": ["mechanical_interface"]
      },
      "artifact_kind": "step",
      "pocket_diameter_tolerance_mm": 0.05,
      "centerline_tolerance_mm": 0.25,
      "pocket_center_tolerance_mm": 0.5,
      "bottom_plane_tolerance_mm": 0.25,
      "axis_dot_min": 0.99
    },
    {
      "id": "bearing_seat_step_presence",
      "kind": "feature_presence",
      "applies_to": {
        "kind": "bearing_seat",
        "intent_any": ["mechanical_interface"]
      },
      "artifact_kind": "step",
      "seat_diameter_tolerance_mm": 0.05,
      "centerline_tolerance_mm": 0.25,
      "seat_center_tolerance_mm": 0.5,
      "shoulder_plane_tolerance_mm": 0.25,
      "axis_dot_min": 0.99
    }
  ]
}
```

Design data can also choose a rulepack beside the artifact:

```json
{
  "schema_version": "burr.design-data.v1",
  "artifact_type": "captured_slider",
  "rulepack": { "path": "../../../rules/captured_slider.rulepack.json" }
}
```

The CLI `--rulepack <file>` flag still overrides this when you want to run a
different rulepack against the same artifact.

Supported rule kinds include:

```txt
hole_edge_distance       -> feature center is far enough from a free edge
feature_edge_distance    -> feature envelope keeps material to a free edge
minimum_wall_thickness   -> enough material remains around a declared hole
feature_presence         -> declared feature has matching STEP evidence
feature_count            -> enough matching declared features exist
numeric_range            -> declared measurement is inside an allowed range
feature_pair_spacing     -> declared slots, holes, or cutouts keep a minimum metadata-based ligament
```

`feature_edge_distance`, `feature_count`, `numeric_range`, and
`feature_pair_spacing` are useful beyond simple screw holes: slots, dense
plates, captured sliders, clearance windows, repeated relief holes or slots,
and other cases where the source emits bounded measurements Burr can check
directly. These are declared design-rule checks, not automatic CAD constraint
solving or stress analysis.

## Versioning

Burr has three versioned surfaces:

```txt
Burr package version       -> CLI/library behavior
Design data schema version -> JSON shape Burr can read
Rulepack schema version    -> rule syntax Burr can execute
```

Receipts include all three:

```json
{
  "schema_version": "burr.receipt.v1",
  "burr_version": "0.27.0",
  "artifact_version": "0.1.0",
  "rulepack_version": "0.14.0",
  "compatibility": {
    "design_data_schema_version": "burr.design-data.v1",
    "rulepack_schema_version": "burr.rulepack.v1"
  }
}
```

Unsupported design data or rulepack schemas fail lint instead of silently producing
untrustworthy receipts.

Legacy `fray-cad.json` files with schema `fray.cad.artifact.v1` are still read
for transition, but new integrations should emit `burr-design-data.json`.

## Repair Loop Proof

Burr's core loop is:

```txt
bad CAD -> Burr check -> explain fix order -> fixed CAD passes
```

Run it with:

```bash
npm run check:repair-loop
```

The bad actuator housing intentionally puts loaded M3 mounting holes too close
to free edges. Burr reports the measured shortage, `burr explain` says what to
fix first, and the fixed housing passes with positive edge-distance margins.

The release gallery also includes a portable repair report:

```txt
repair-reports/actuator-housing-edge-distance.json
repair-reports/actuator-housing-edge-distance.md
```

That report links the bad receipt, measured failures, first fix, generated
`repair_actions[]`, and fixed passing receipt. The repair actions are
receipt/design-data suggestions only; Burr does not auto-edit CAD. Agents and
websites can render the repair proof without scraping terminal output.

Since Burr 0.16, each action gives the failing feature, action kind, checked
parameter, suggested feature-center movement, measured/required/margin evidence,
failure reason, and the fixed after-feature that verifies the suggestion.

In Burr 0.18, the repair action contract also includes a required `source_hint`
with the source file path, edit kind, selector, exact before/after source text,
editable value path, before/after design-data values,
`exact_from_design_data` confidence, and a short rationale. This is an edit hint
only; Burr still does not auto-edit CAD. Agent repair runners should iterate
generate/check/explain-json, apply only exact source hints, and stop when the
part passes or the packet no longer contains an exact source edit.

## Example Result

Before repair, the actuator CAD is bad:

```txt
FAIL examples/build123d-actuator-housing-repair/bad/burr-design-data.json -> <not written>

4 problems:
1. M3 loaded hole m3_front_left is too close to the edge.
   Measured center-to-edge: 8 mm
   Required center-to-edge: 10.2 mm
   Short by: 2.2 mm
   Try moving the hole inward or increasing the surrounding part size.
```

`burr explain` turns the failed receipt into plain repair guidance: fix stale or
missing artifacts first if they exist, then fix unsafe dimensions such as a
loaded M3 hole near an edge.

After repair, the actuator CAD passes:

```json
{
  "status": "pass",
  "measured": {
    "center_to_edge_mm": 12,
    "wall_to_edge_mm": 10.3
  },
  "required": {
    "center_to_edge_mm": 10.2,
    "wall_to_edge_mm": 8.5
  },
  "margin_mm": 1.8
}
```

Thin wall fixture:

```txt
FAIL examples/build123d-wall-thickness/bad/burr-design-data.json -> <not written>

1 problem:
1. M3 clearance hole m3_alignment leaves too little wall.
   Measured wall thickness: 1.2 mm
   Required wall thickness: 2 mm
   Short by: 0.8 mm
   Try moving the hole inward or increasing part width.
```

Missing STEP feature fixture:

```txt
FAIL examples/build123d-step-presence/bad/burr-design-data.json -> <not written>

1 problem:
1. Declared clearance hole m3_claimed is missing from the STEP artifact.
   Checked artifact: presence.step
   Candidate cylinders found: 0
   Regenerate the STEP from the same helper that emitted the design data.
```

`Candidate cylinders found` and `Candidate planes found` are not counts of
failed features. They are the STEP faces Burr considered while trying to prove
one declared feature. Extra faces are ignored unless a rulepack selects matching
declared intent and the geometry fits the declared tolerances.

## Status

Early prototype. Current checks combine design-data rules with narrow STEP
feature-presence verification for declared M3 clearance holes, declared
straight slots, declared counterbores, declared heat-set insert pockets, and
declared bearing seats. They also check declared edge-material envelopes for
loaded holes, straight slots, counterbores, and loaded bearing seats. Burr does
not classify all holes, slots, counterbores, pockets, or seats in a model or
decide which features matter.

Ligament checks use declared feature metadata selected by a rulepack. Burr does
not search the whole model for every thin region, infer load paths, or certify
that the remaining material survives use.

By default, the Rust CLI reads simple analytic STEP cylinder entities directly.
For stronger local verification, install the optional Python/OCP workspace and
run with:

```bash
BURR_STEP_CYLINDER_BACKEND=ocp \
BURR_OCP_STEP_CYLINDERS="uv run --package burr-ocp burr-ocp-step-cylinders" \
burr check .
```

The OCP helper extracts measured cylinder and plane candidates. Burr still owns
rule matching, diagnostics, and receipts.
