# Install Burr

Burr has two pieces:

```txt
@fraylabs/burr   Node CLI and linter
burr-build123d   Python helper that emits burr-design-data.json from build123d
```

Neither package is published to npm or PyPI yet. Install from GitHub or a local
checkout for now.

## CLI From GitHub

Install the Burr CLI globally:

```bash
npm install -g github:fraylabs/burr
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
```

Run checks:

```bash
npm run check
npm run check:build123d
```

Use the local CLI without global install:

```bash
node bin/burr.mjs --version
node bin/burr.mjs check examples/linear-actuator-good
```

Or link the CLI while developing:

```bash
npm link
burr --version
```

## build123d Helper With uv

Inside a Burr checkout, `burr-build123d` is a uv workspace package.

Run the example design files through uv:

```bash
uv sync --all-packages
uv run --package burr-build123d python examples/build123d-actuator/good/design.py
node bin/burr.mjs check examples/build123d-actuator/good
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
npm package: not published
PyPI package: not published
GitHub source install: supported
uv local workspace install: supported
```

Planned package names:

```txt
npm:  @fraylabs/burr
PyPI: burr-build123d
```
