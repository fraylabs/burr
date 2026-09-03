# How Burr Works

Burr opens ordinary CAD files locally:

```txt
burr .
  -> discover STEP, STL, and GLB files
  -> preserve their folder hierarchy
  -> compile the selected model with Look
  -> render it in a local browser
  -> animate configured rigid assembly poses
  -> inspect STEP assembly component interference
  -> refresh when the source file changes
```

No upload, generated manifest, or Burr-specific CAD helper is required.

## Optional project scope

Without configuration, the folder passed to `burr` is the model root. A project
can use `.burr/config.toml` to name one or more model folders while retaining a
stable project root:

```toml
schema_version = "burr.project.v1"

[project]
models = ["models"]
```

The project contract can also name rigid motions between two STEP assembly
poses. See
[project configuration](project-configuration.md) for its validation and
path-safety rules.

## Named assembly motion

A project may connect two STEP files as the endpoints of a named motion. Burr
matches uniquely named components, verifies that their geometry is unchanged,
and interpolates their transforms locally. The viewer then provides one
play/pause button and a scrubber between the endpoint labels. This is intended
for mechanisms such as a deployed and folded hanger, not mesh morphing or
physics simulation.

The browser receives precomputed rigid-transform frames; model vertices are not
duplicated for every frame. **Snapshot** captures whichever frame is currently
visible.

## Snapshots

**Snapshot** exports the current model canvas as a PNG through the browser's
download flow. The image preserves the active camera, light or dark theme, and
X-ray or Solid mode. Burr does not write the image into the model workspace or
send model geometry to a remote renderer.

## Geometry-native assembly interference

Look preserves each STEP assembly occurrence's component name, mesh, and world
transform. Burr checks those world-space component pairs with a strict bounds
filter followed by mesh interference and containment evidence. Face contact is
not interference.

The Checks tab reports `pass`, `fail`, or `incomplete`. Selecting a finding
highlights its two component occurrences in the same Look viewport. The viewer
defaults to X-ray so enclosed or occluded components remain visible and offers
Solid mode for ordinary inspection. The result is cached with the compiled
model and replaced when the source version changes.

A clean result requires a STEP assembly with at least two closed component
meshes. Burr does not currently compute exact Boolean overlap volume, and it
does not turn an unsupported or inconclusive model into a pass.

See the [roadmap](roadmap.md) for the current boundary and deferred work.
