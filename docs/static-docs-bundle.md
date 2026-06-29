# Fray Static Docs Bundle Contract

This contract lets fray-site render Burr documentation from a release artifact
without checking out the Burr repository or regenerating CAD.

## Source

```txt
repo: fraylabs/burr
release_tag: burr-v0.29.0
asset_name: burr-docs-v0.29.0.zip
asset_url: https://github.com/fraylabs/burr/releases/download/burr-v0.29.0/burr-docs-v0.29.0.zip
```

The website should treat Burr release assets as read-only product data.

## Build

```bash
npm run docs:artifact
npm run check:docs:artifact
```

`docs/static-docs-manifest.json` is the Burr-owned source manifest. The build
script copies those Markdown and reference files into a versioned release
folder, writes a generated manifest with hashes, and zips the folder.

## Zip Layout

```txt
burr-docs-v0.29.0/
  README.md
  manifest.json
  markdown/
    how-it-works.md
    reference/
      design-data.md
      receipt.md
      cli.md
      rulepack.md
    overview.md
    install.md
    changelog.md
    fray-gallery-contract.md
    fray-static-docs-contract.md
    gallery.md
    burr-build123d.md
    burr-ocp.md
  reference/
    package.json
    LICENSE
    rules/
      actuator_mount.rulepack.json
      printed_plate.rulepack.json
      captured_slider.rulepack.json
      hardware_fit.rulepack.json
      tool_access.rulepack.json
      mount_pattern.rulepack.json
      printable_retention.rulepack.json
      boss_support.rulepack.json
```

## Manifest

Manifest path:

```txt
burr-docs-v0.29.0/manifest.json
```

Schema:

```json
{
  "schema_version": "burr.docs-artifact.v1",
  "burr_version": "0.29.0",
  "artifact_id": "burr-docs-v0.29.0",
  "generated_at": "ISO-8601 timestamp",
  "source": {
    "repository": "fraylabs/burr",
    "tag": "burr-v0.29.0"
  },
  "documents": [
    {
      "title": "Burr Overview",
      "kind": "overview",
      "content_type": "text/markdown; charset=utf-8",
      "source_path": "README.md",
      "bundle_path": "markdown/overview.md",
      "sha256": "hex sha256",
      "size_bytes": 12345
    },
    {
      "title": "Design Data Reference",
      "kind": "site_page",
      "content_type": "text/markdown; charset=utf-8",
      "source_path": "docs/site/reference/design-data.md",
      "bundle_path": "markdown/reference/design-data.md",
      "slug": "reference/design-data",
      "section": "Reference",
      "nav_order": 110,
      "sha256": "hex sha256",
      "size_bytes": 12345
    }
  ],
  "references": [
    {
      "title": "Actuator Mount Rulepack",
      "kind": "rulepack",
      "content_type": "application/json",
      "source_path": "rules/actuator_mount.rulepack.json",
      "bundle_path": "reference/rules/actuator_mount.rulepack.json",
      "sha256": "hex sha256",
      "size_bytes": 12345
    }
  ]
}
```

## Consumer Rules

- Use `manifest.json` as the index. Do not depend on a directory crawl.
- Website pages are manifest documents with `kind: "site_page"` and a `slug`.
  fray-site renders those under `/burr/<slug>`.
- Render files from `bundle_path`; use `source_path` only for attribution back
  to the Burr repository.
- Treat Markdown files as static content. Do not execute code blocks.
- Treat reference JSON as display or schema/reference data. It is not a proof
  artifact.
- Gallery receipts remain the proof artifacts for mechanical examples. This
  docs bundle can link to gallery contracts, but it must not replace the
  gallery artifact when fray-site needs receipt-backed examples.
