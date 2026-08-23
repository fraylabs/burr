# Burr Roadmap

Burr is moving toward geometry-native CAD inspection: open ordinary model
files first, derive facts from their actual geometry, and require no generated
metadata for the basic experience.

## Now

- `burr .` opens STEP, STL, and GLB files locally.
- The sidebar mirrors nested model folders and refreshes the active model when
  its source changes.
- Optional `.burr/config.toml` limits the folders Burr treats as model roots.

## Next: geometry-native checks

The first complete vertical slice is assembly interference:

```text
open a STEP assembly
  -> identify its bodies or components
  -> detect overlapping solid volume
  -> report the involved geometry and measured overlap
  -> select the finding in the viewer
```

Only the minimum body/component model needed for that proof should be built.
Clearance, distance, and thin-region checks can follow once Burr has stable
geometry references and viewer selection.

## Later

Declared-intent checks such as mechanical fit, process-specific design rules,
and project-authored rulepacks may return after geometry-native inspection is
useful on ordinary CAD files. They should add semantic intent where geometry
alone is insufficient, not become a prerequisite for opening or checking a
project.

The receipt-based `burr check` workflow from Burr 0.30 remains available as a
published compatibility surface. It is isolated from the new workbench and is
not the foundation for geometry-native checks.

## Not now

- ISO certification claims
- FEA or stress simulation
- A generic assembly framework before the interference proof needs it
- A new pack schema before a geometry-native check is working end to end
