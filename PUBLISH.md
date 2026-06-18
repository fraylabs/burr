# Publishing Burr Packages

## Rust CLI

The Rust CLI is published to crates.io as `burr`.

```bash
cargo publish
```

## Python build123d Helper

The Python helper is published to PyPI as `burr-build123d`.

Preferred path: GitHub trusted publishing.

Configure PyPI with a pending publisher:

```txt
PyPI project: burr-build123d
Owner: fraylabs
Repository: burr
Workflow: publish-python.yml
Environment: pypi
```

Then run the `Publish Python` GitHub Actions workflow with:

```txt
ref: burr-build123d-v0.5.0
```

Local token fallback:

```bash
export UV_PUBLISH_TOKEN=pypi-...
uv build --package burr-build123d
uv publish dist/burr_build123d-0.5.0*
```

After publishing, verify from a fresh project:

```bash
uv init /tmp/burr-pypi-install-check
cd /tmp/burr-pypi-install-check
uv add burr-build123d
uv run python -c "import burr_build123d; print(burr_build123d.__version__)"
```
