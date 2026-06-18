# Install Burr

Burr has two pieces:

```txt
burr             Rust CLI and linter
burr-build123d   Python helper that emits burr-design-data.json from build123d
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
PyPI package: not published
uv local workspace install: supported
```

Planned package names:

```txt
PyPI: burr-build123d
```
