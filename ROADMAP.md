# Burr Roadmap

Burr is moving toward geometry-native CAD inspection: open ordinary model
files first, derive facts from their actual geometry, and require no generated
metadata for the basic experience.

## Now

- `burr .` opens STEP, STL, and GLB files locally.
- The sidebar mirrors nested model folders and refreshes the active model when
  its source changes.
- Optional `.burr/config.toml` limits the folders Burr treats as model roots.
- STEP assemblies with at least two component occurrences receive one
  geometry-native interference check:

```text
open a STEP assembly
  -> identify its bodies or components
  -> detect crossing surfaces, containment, or coincident occurrences
  -> distinguish face contact from interference
  -> report the involved components
  -> highlight the selected pair in the viewer
```

The check returns `incomplete` instead of a clean result when the model is not a
supported STEP assembly or component meshes are not closed. It does not claim
exact Boolean overlap volume; surface-crossing witnesses and containment are
the current evidence boundary.

## Next

Use the interference proof on representative real assemblies and harden only
the failure modes that evidence exposes. Clearance, distance, and thin-region
checks can be considered after the component references and selection loop hold
up outside the fixtures.

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
- Exact overlap-volume computation without a proven Boolean geometry backend
- A generic check framework or pack schema before a second native check needs it
