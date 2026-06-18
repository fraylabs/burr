# Burr V1 DX Goal

## Fit

Use Goal mode for this work. The outcome needs repeated design, implementation,
fixture expansion, and verification, and success can be checked by CLI commands,
example projects, receipts, and clean user-facing diagnostics.

No goal is active from this file alone.

## Outcome

A new build123d user can create a simple mechanical part, emit Burr design data
without hand-writing JSON, run Burr, and understand exactly what to fix when a
basic mechanical design rule fails.

The trusted v1 claim is:

```txt
For supported build123d helper features and Burr rulepacks, Burr can check
generated design data and produce clear diagnostics that catch known mechanical
mistakes before print.
```

## Baseline

Observed current state:

- Burr v0.2.0 exists as a public Node CLI package.
- Burr reads `burr-design-data.json` and writes `burr-receipt.json`.
- Legacy `fray-cad.json` is still accepted for transition.
- Current examples prove one actuator mount rule:
  - bad actuator fails edge-distance lint;
  - good actuator passes;
  - stale source hash fails.
- The current workflow still expects design data JSON to exist before Burr runs.

Current weak point:

```txt
Users should not have to hand-author burr-design-data.json for normal build123d
workflows.
```

## Scope

Build v1 DX around the smallest useful loop:

```txt
build123d design file
  -> generated STEP or part artifact
  -> generated burr-design-data.json
  -> Burr pretty diagnostics
  -> burr-receipt.json
```

Required v1 pieces:

- A minimal `burr-build123d` Python helper package or folder.
- A design-data recorder usable from normal build123d files.
- At least one helper that creates geometry and records intent together:
  `m3_clearance_hole` or equivalent.
- Pretty CLI diagnostics for the existing hole edge-distance rule.
- A clean example that a new user can run without reading schema docs first.
- A negative fixture proving the bad actuator still fails.
- A positive fixture proving the fixed actuator still passes.
- Documentation that uses these terms:
  - design file;
  - generated part;
  - design data;
  - receipt.

## Non-Scope

Do not build:

- a CAD kernel;
- a replacement for build123d;
- a new CAD language;
- STEP topology extraction;
- FEA or FEM;
- automatic geometry fixing;
- a visual editor;
- universal rule coverage.

Do not make Burr core depend on build123d objects. Burr core owns:

```txt
burr-design-data.json -> rules -> diagnostics -> receipt
```

The build123d adapter owns:

```txt
build123d helper calls -> geometry + burr-design-data.json
```

## Primary Verifier

From a clean checkout of `fraylabs/burr`, these commands must pass:

```bash
npm run check
node bin/burr.mjs --version
node bin/burr.mjs check examples/linear-actuator-bad --no-write-receipt
node bin/burr.mjs check examples/linear-actuator-good --no-write-receipt
```

Expected result:

- `npm run check` exits 0.
- `--version` prints the package version.
- bad example exits non-zero and reports the specific failing feature.
- good example exits 0.

## Supporting Verifiers

Add v1-specific checks before completion:

```bash
python -m pytest
python examples/build123d-actuator/design.py
node bin/burr.mjs check examples/build123d-actuator/bad --no-write-receipt
node bin/burr.mjs check examples/build123d-actuator/good --no-write-receipt
```

If Python packaging is not introduced yet, replace `python -m pytest` with the
narrowest equivalent command that tests the build123d adapter.

The build123d example must produce:

```txt
burr-design-data.json
generated STEP or declared generated artifact
burr-receipt.json when check writes receipts
```

## Diagnostics Standard

A v1 failure must be understandable without reading the schema.

Bad:

```txt
insufficient_edge_distance
```

Good:

```txt
M3 loaded hole m3_lower_left is too close to the edge.
Measured center-to-edge: 8.0 mm
Required center-to-edge: 10.2 mm
Short by: 2.2 mm
Try moving the hole inward or increasing the surrounding part size.
```

Machine-readable receipt fields must still exist. Human output is additive, not
a replacement.

## Iteration Loop

1. Inspect current Burr CLI, examples, rulepack, and tests.
2. Add or change one meaningful DX surface.
3. Run the primary verifier.
4. Run the relevant adapter/example verifier.
5. Record failing command and exact reason.
6. Fix the smallest cause.
7. Repeat until the completion proof passes from clean state.

## Anti-Cheating Constraints

Do not:

- weaken the existing bad actuator fixture;
- remove stale hash checks;
- remove legacy `fray-cad.json` support without explicit approval;
- hide failures behind warnings;
- claim build123d support if the example does not actually import and use
  build123d;
- make users hand-write design data in the “easy path” example;
- make Burr core import build123d.

## Review Pressure

Before completion, perform a skeptical pass:

- Can a user run the example without knowing the schema?
- Does the bad fixture fail for the right mechanical reason?
- Does the good fixture pass for the right reason?
- Does the diagnostic tell the user what to change?
- Is `burr-design-data.json` still the stable adapter boundary?
- Could build123d be replaced later without rewriting Burr core?

If subagents are authorized, use one independent verification lane to run the
example from fresh checkout context and report only commands, outputs, and
confusing steps.

## Blocker Standard

A blocker requires concrete evidence:

- missing build123d installation path that cannot be installed in the available
  environment;
- package manager failure with exact command output;
- incompatible runtime version with exact error;
- external credential or network requirement not needed for local verification.

Difficulty, uncertain API shape, or messy code is not a blocker.

## Completion Proof

The goal is complete only when all are true:

- Burr has a build123d adapter/helper path.
- A new example uses build123d helpers to emit `burr-design-data.json`.
- The example has a failing and passing fixture.
- CLI output is human-readable enough to fix the failing fixture.
- Machine-readable receipts still contain measured/required/margin data.
- The primary and supporting verifier commands pass with recorded outputs.
- README explains the v1 loop before showing schema details.

## Exact Objective For Activation

```txt
Complete and verify the Burr V1 DX objective defined in /Users/brianlim/coding/burr/GOAL.md.
```

Activation state: drafted.
