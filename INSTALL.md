# Install Burr

Burr has two pieces:

```txt
burr             Rust CLI and linter
burr-build123d   Python helper that emits burr-design-data.json from build123d
burr-ocp        Optional Python/OpenCascade STEP geometry extractor
```

The CLI is Rust-first. The Python helper is managed with uv.

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

Start a build123d part:

```bash
burr init my-part
cd my-part
uv run python design.py
burr check .
```

The generated project installs `burr-build123d==0.5.0` from PyPI.

## Local Development Checkout

Clone and install:

```bash
git clone https://github.com/fraylabs/burr.git
cd burr
npm install
uv sync --all-packages
cargo test
```

Run checks:

```bash
npm run check
npm run check:build123d
npm run check:ocp
npm run check:mixed-intent
npm run check:counterbore
npm run check:slots
```

Use the local CLI without global install:

```bash
cargo run -- --version
cargo run -- check examples/linear-actuator-good
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
PyPI package: burr-build123d==0.5.0
uv local workspace install: supported
```

Planned package names:

```txt
PyPI: burr-build123d
PyPI: burr-ocp
```
