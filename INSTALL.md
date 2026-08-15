# Install Burr

Burr has three runtime pieces:

```txt
burr             Rust CLI and linter
burr-build123d   Python helper that emits burr-design-data.json from build123d
burr-ocp        Optional Python/OpenCascade STEP geometry extractor
```

The CLI is Rust-first. The Python helper is managed with uv. Burr's public npm
package is a separate open-source distribution surface described below; it does
not replace the Rust CLI.

## Rust CLI From crates.io

Install the Burr CLI with Cargo:

```bash
cargo install burr
```

Check it:

```bash
burr --version
```

Run it on a folder containing `burr-design-data.json`:

```bash
burr check path/to/design-folder
```

The design data must select a rulepack with a built-in string such as
`"builtin:actuator_mount"` or a `rulepack.path`, or the command must receive
`--rulepack <selector>`. Burr does not silently choose a default rulepack.

`burr check` exits `0` for `pass`, `3` for `incomplete`, `1` for `fail`, and `2`
when invocation, input reading, or rulepack selection prevents a receipt-backed
run. Treat `incomplete` as a blocked trust claim, not as a successful check.

Start a build123d part:

```bash
burr init my-part
cd my-part
uv run python design.py
burr check .
burr explain .
```

The generated project installs `burr-build123d==0.10.0` from PyPI.

To verify the published install path from a Burr checkout:

```bash
npm run check:fresh-install
```

That check proves the starter loop end to end:

```txt
cargo install published Burr
  -> burr init
  -> uv run python design.py
  -> burr check passes
  -> move the starter M3 hole toward the side edge
  -> burr check fails on edge distance
  -> burr explain reports measured 4 mm vs required 10.2 mm
  -> restore the starter
  -> burr check passes
```

## Public npm Distribution

`@fraylabs/burr` is intentionally prepared as a future public open-source npm
distribution. Its package manifest defines a source tarball containing Burr's
source, rulepacks, machine-readable schemas, documentation, example source, and
Node-based development and artifact tooling. It is not published yet and is not
the executable installation path: install the `burr` CLI from crates.io and the
optional build123d helper from PyPI.

The manifest sets `publishConfig.access` to `public`. `npm run check:package`
runs `npm pack --dry-run` to verify the future tarball without publishing it.
That future surface is an OSS distribution contract, not an accidental side
effect of Burr's local npm scripts.

## Local Development Checkout

Clone and set up:

```bash
git clone https://github.com/fraylabs/burr.git
cd burr
uv sync --all-packages
cargo test --locked
```

Run checks:

```bash
npm run check
npm run check:build123d
npm run check:ocp
npm run check:mixed-intent
npm run check:bearing-seat
npm run check:bearing-seat-edge-distance
npm run check:counterbore
npm run check:insert-pocket
npm run check:insert-pocket-edge-distance
npm run check:standoff-boss-edge-distance
npm run check:slots
npm run check:mistake-library-v1
npm run check:gallery
npm run check:gallery:render
npm run check:gallery:artifact
npm run check:docs:artifact
npm run check:explain
npm run check:release-candidate
npm run check:fresh-install
```

Use the local CLI without global install:

```bash
cargo run -- --version
cargo run -- check examples/linear-actuator-good
cargo run -- explain examples/linear-actuator-good
```

Or install the local Rust CLI while developing:

```bash
cargo install --path .
burr --version
```

## build123d Helper With uv

Inside a Burr checkout, `burr-build123d` is a uv workspace package.

Run the example design files through uv:

```bash
uv sync --all-packages
uv run --package burr-build123d python examples/build123d-actuator/good/design.py
cargo run -- check examples/build123d-actuator/good
npm run check:gallery
```

For your own local script in the same checkout:

```bash
uv run --package burr-build123d python path/to/design.py
```

## Optional OpenCascade STEP Backend

The Rust CLI works without Python/OCP by default. For stronger local STEP
cylinder and plane extraction, use the optional `burr-ocp` workspace package:

The OCP backend may find many cylindrical and planar faces in a STEP file. Burr
still applies rulepack intent first, then uses those faces only as evidence for
declared features.

```bash
uv sync --all-packages
uv run --package burr-ocp burr-ocp-step-cylinders path/to/part.step
```

To make `burr check` use that extractor:

```bash
BURR_STEP_CYLINDER_BACKEND=ocp \
BURR_OCP_STEP_CYLINDERS="uv run --package burr-ocp burr-ocp-step-cylinders" \
cargo run -- check path/to/design-folder
```

## Install the Python Helper From Local Path

For another project on the same machine:

```bash
uv add --editable /path/to/burr/packages/burr-build123d
```

Then your build123d design can import:

```python
from burr_build123d import BurrDesignData, m3_clearance_hole
```

## Current Publish Status

```txt
Rust CLI: published to crates.io as burr
PyPI package: burr-build123d==0.10.0
Planned public npm package: @fraylabs/burr (not published yet)
uv local workspace install: supported
```

Planned package names:

```txt
PyPI: burr-build123d
PyPI: burr-ocp
```
