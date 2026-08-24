# Burr Documentation

Burr is a fast, local browser for STEP, STL, and GLB files with a
geometry-native STEP assembly interference check.

## Quick start

Install Burr from its public Git repository:

```bash
cargo install --git https://github.com/fraylabs/burr.git --locked
```

Open a folder containing CAD models:

```bash
cd your-project
burr .
```

The sidebar discovers supported models recursively and refreshes the active
model when its source file changes. Models stay on your machine.

## Read next

- [How Burr works](how-it-works.md)
- [Project configuration](project-configuration.md)
- [CLI reference](reference/cli.md)
- [Roadmap](roadmap.md)

The [design system](design-system.md) documents the shared visual language of
the Burr shell and Look viewport.
