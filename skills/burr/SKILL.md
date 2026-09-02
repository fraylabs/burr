---
name: burr
description: Create, open, inspect, and check physical designs through Burr. Use when working in Burr, running burr on a model folder, generating or modifying CAD for Burr, browsing STEP/STL/GLB files, or inspecting STEP assembly interference. Existing-file workflows require no external design provider.
---

# Burr design environment

Use Burr as the user-facing environment for a physical-design task. Burr owns
the project loop: understand the requested outcome, keep work in the user's
project, route source creation or editing to the appropriate optional provider,
then open and inspect the resulting artifacts.

The `burr` CLI itself remains a fast local file browser and geometry checker.
It must work on existing STEP, STL, or GLB files without CAD, KiCad, or another
provider installed.

## Choose the path from the request

### Existing model

When the user wants to open, browse, compare, or check existing supported
files, use Burr directly. Do not load or install a design provider.

```bash
burr path/to/model-folder
```

From inside a project, the usual command is `burr .`.

### Create or modify a design

When source geometry must change, read
[`references/providers.md`](references/providers.md). Select only the provider
that owns the requested source domain, let it create and validate that source,
then return to Burr with its exported model.

Examples:

- A new clothes hanger uses a mechanical CAD provider, then Burr opens the
  resulting model. It does not require an electronics provider.
- A PCB change uses a KiCad provider, which retains ownership of the KiCad
  project and exports a populated STEP only when mechanical review needs it.
- A robot enclosure containing a PCB can use both providers, each for its own
  source, before Burr inspects the combined mechanical assembly.

Do not install a missing provider without the user's authority. Do not copy an
upstream provider's instructions into this skill.

## Start and refresh Burr

Confirm the installed CLI before describing its output as Burr evidence:

```bash
burr --version
```

Start Burr at the narrowest useful project folder after a viewable artifact
exists. Burr discovers STEP/STP, STL, and GLB recursively, preserves folder
hierarchy in the sidebar, and watches the selected model for changes.

Use `.burr/config.toml` only to limit model roots when unconfigured discovery
is noisy. Provider selection is not currently a Burr project-config feature;
do not invent provider keys or rulepacks.

## Inspect or repair an assembly

Read [`references/inspection-loop.md`](references/inspection-loop.md) when the
task needs STEP assembly-interference evidence or a repair driven by a Burr
finding.

The Checks outcomes mean:

- `pass`: Burr completed the supported component-pair check and found no
  interference.
- `fail`: Burr found one or more interfering component pairs.
- `incomplete`: Burr cannot support a clean claim for this input.

A clean-looking image never overrides `fail` or `incomplete`. A `pass` does not
prove production fit, clearance, tolerances, motion, strength, or
manufacturability.

## Ownership boundaries

- Burr owns model discovery, local viewing, refresh, and its reported checks.
- A provider owns its editable source, domain validation, and exports.
- Fix the authoritative source; never hand-edit generated STEP/STL/GLB output.
- Keep ordinary viewing independent of every optional provider.
- Use the minimum provider set needed for the user's request.
- Do not revive removed Burr rulepack, receipt, or generated-metadata
  workflows.

Final responses should name the authoritative source, produced model path,
Burr version, checks outcome when used, provider validation when relevant, and
any unsupported or inconclusive scope.
