# Performance Evidence

Burr uses the published
[fold-flat travel hanger](https://github.com/fraylabs/possible-outcomes/tree/main/outcomes/fold-flat-travel-hanger)
and
[digital photo frame](https://github.com/fraylabs/possible-outcomes/tree/main/outcomes/digital-photo-frame)
as real-world loading fixtures. They exercise named motion between two STEP
assemblies and are intentionally much larger than the repository's unit-test
models.

## Burr 0.33.0 result

Measured on 2026-09-03 with the optimized `aarch64-apple-darwin` binary on an
Apple M1 Pro running macOS 26.5.1:

| Outcome motion | Cold generation | Same process | After restart | Viewer response |
| --- | ---: | ---: | ---: | ---: |
| Fold-flat hanger | 4,336.76 ms | 5.71 ms | 6.59 ms | 2,945,235 bytes |
| Digital photo frame | 9,006.08 ms | 6.39 ms | 8.09 ms | 4,078,500 bytes |

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

Extract the Burr projects from the two outcome packs, then run:

```bash
cargo build --release --locked
npm run measure:outcomes -- /path/to/hanger-project /path/to/photo-frame-project
```

The harness creates a fresh isolated cache for each project, requests its first
configured motion, repeats the request in the same process, restarts Burr, and
requires the third request to report a disk-cache hit. It drains Look's
diagnostic output so a full stderr pipe cannot distort tessellation timing.

The normal repository gate uses small checked-in STEP fixtures to prove cache
reuse and source invalidation without downloading the external outcome packs.
