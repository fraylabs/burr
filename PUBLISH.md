# Publishing Burr Packages

## Rust CLI

The Rust CLI is published to crates.io as `burr`.

```bash
cargo publish
```

## Python build123d Helper

The Python helper is published to PyPI as `burr-build123d`.

Preferred path: GitHub trusted publishing.

Configure PyPI with a trusted publisher. For a first publish, PyPI calls this a
pending publisher until the workflow succeeds once.

```txt
PyPI project: burr-build123d
Owner: fraylabs
Repository: burr
Workflow: publish-python.yml
Environment: pypi
```

PyPI must match these GitHub trusted-publishing claims:

```txt
sub: repo:fraylabs/burr:environment:pypi
repository: fraylabs/burr
repository_owner: fraylabs
job_workflow_ref: fraylabs/burr/.github/workflows/publish-python.yml@refs/heads/main
ref: refs/heads/main
environment: pypi
```

Then run the `Publish Python` GitHub Actions workflow against the matching
package tag:

```bash
gh workflow run "Publish Python" --repo fraylabs/burr -f ref=burr-build123d-v0.9.0
gh run list --repo fraylabs/burr --workflow "Publish Python" --limit 1
```

If publish fails with `invalid-publisher`, the PyPI pending publisher does not
match the claims above.

Local token fallback, for emergencies only:

```bash
cp .env.local.example .env.local
# Replace the placeholder with the real PyPI API token.
npm run publish:python:local
npm run publish:python:local -- --confirm
```

The local helper refuses to publish unless `UV_PUBLISH_TOKEN` is set to a real
`pypi-...` token and `--confirm` is passed.

After publishing, verify from a fresh project:

```bash
uv init /tmp/burr-pypi-install-check
cd /tmp/burr-pypi-install-check
uv add burr-build123d
uv run python -c "import burr_build123d; print(burr_build123d.__version__)"
```
