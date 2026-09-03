# Performance Evidence

Burr uses the published
[fold-flat travel hanger](https://github.com/fraylabs/possible-outcomes/tree/main/outcomes/fold-flat-travel-hanger)
and
[digital photo frame](https://github.com/fraylabs/possible-outcomes/tree/main/outcomes/digital-photo-frame)
as real-world loading fixtures. They exercise named motion on STEP assemblies
and are intentionally much larger than the repository's unit-test models.

## Burr 0.34.0 result

Measured on 2026-09-03 with the optimized `aarch64-apple-darwin` binary on an
Apple M1 Pro running macOS 26.5.1. These are the medians of three isolated
runs:

| Outcome motion | Cold generation | Same process | After restart | Viewer response |
| --- | ---: | ---: | ---: | ---: |
| Fold-flat hanger | 1,832.98 ms | 6.73 ms | 8.80 ms | 2,945,519 bytes |
| Digital photo frame | 4,391.44 ms | 5.97 ms | 8.23 ms | 4,078,771 bytes |

Version 0.34.0 reads and tessellates the deployed assembly once, then applies
the declared revolute and prismatic joints. Against the 0.33.0 measurements
below, that reduced median cold response time by about 50% for the hanger and
48% for the frame. The output still contains the complete animation; only the
redundant second STEP compilation was removed.

## Burr 0.33.0 result

Measured on 2026-09-03 with the optimized `aarch64-apple-darwin` binary on an
Apple M1 Pro running macOS 26.5.1:

| Outcome motion | Cold generation | Same process | After restart | Viewer response |
| --- | ---: | ---: | ---: | ---: |
| Fold-flat hanger | 3,672.72 ms | 6.09 ms | 8.29 ms | 2,945,540 bytes |
| Digital photo frame | 8,414.08 ms | 6.47 ms | 6.93 ms | 4,078,805 bytes |

The cache provenance reported by Burr was `generated`, `memory`, and `disk`
for the three columns respectively. The automated viewer proof separately
changes a STEP source and requires the next response to be regenerated.

For comparison, the Burr 0.32.0 reproduction on the same machine took
4,822.23 ms cold and 17.63 ms warm for the hanger, and 10,146.50 ms cold and
38.17 ms warm for the photo frame. Version 0.32.0 had no cross-process viewer
reuse, so restarting returned to cold generation.

These numbers time the complete local `/viewer` response, including reading
its body. They are evidence for this machine and these outcome packs, not a
universal latency promise. Browser parsing and the first WebGL frame add
device-dependent time. During every cold run, the workbench now reports the
actual source pose and preparation stage instead of displaying an unexplained
spinner.

## Reproduce

Extract the Burr projects from the two outcome packs and migrate their motion
configuration to `burr.project.v2` as described in
[project configuration](project-configuration.md), then run:

```bash
cargo build --release --locked
npm run measure:outcomes -- /path/to/hanger-project /path/to/photo-frame-project
```

The 0.34.0 runs use the published source assemblies and their authoritative CAD
joint datums: the hanger's two arm hinges, hook hinge, and 7.5 mm lock-bar
travel; and the frame's easel hinge. The harness creates a fresh isolated cache
for each project, requests its first configured motion, repeats the request in
the same process, restarts Burr, and requires the third request to report a
disk-cache hit. It drains Look's diagnostic output so a full stderr pipe cannot
distort tessellation timing.

The normal repository gate uses small checked-in STEP fixtures to prove cache
reuse and source invalidation without downloading the external outcome packs.
