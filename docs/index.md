# Burr Documentation

Burr is a fast, local browser for STEP, STL, and GLB files with a
geometry-native STEP assembly interference check.

The optional `$burr` agent skill can coordinate source creation through an
independently installed design provider before returning to Burr for local
review. Providers are optional; ordinary `burr .` usage has no CAD or KiCad
skill dependency.

## Quick start

Install Burr from its public Git repository:

On Apple Silicon:

```bash
cargo install --git https://github.com/fraylabs/burr.git --tag burr-v0.35.0 --locked
```

On x86-64, AVX and FMA CPU support is required by Look's interval-math kernel:

```bash
RUSTFLAGS="-Ctarget-feature=+avx,+fma" cargo install --git https://github.com/fraylabs/burr.git --tag burr-v0.35.0 --locked
```

These flags are necessary for Git installs; local source builds use the
repository's `.cargo/config.toml`. Older CPUs without AVX/FMA are unsupported.

Open a folder containing CAD models:

```bash
cd your-project
burr .
```

The sidebar discovers supported models recursively and refreshes the active
model when its source file changes. Models stay on your machine.

## Read next

- [How Burr works](how-it-works.md)
- [Performance evidence](performance.md)
- [Project configuration](project-configuration.md)
- [CLI reference](reference/cli.md)
- [Roadmap](roadmap.md)

The [design system](design-system.md) documents the shared visual language of
the Burr shell and Look viewport.
