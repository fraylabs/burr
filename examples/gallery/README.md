# Burr Example Gallery

These examples are small printable parts, not abstract fixtures. Each `design.py`
generates:

- a STEP artifact
- `burr-design-data.json`
- a Burr receipt from `burr check`

Run:

```bash
npm run check:gallery
```

The generated CAD artifacts are ignored by git. The source files are the stable
CAD contract; the receipts are regenerated to prove that Burr can connect the
declared mechanical intent to exported STEP geometry.

## Burr 0.14 Repair Story

The 0.14 gallery narrative should explain the actuator repair loop in simple
terms:

```txt
bad CAD -> Burr check -> explain fix order -> fixed CAD passes
```

The bad actuator is the before state: Burr catches a declared mechanical mistake
and reports measured evidence. The fixed actuator is the after state: the same
intent is repaired and the receipt passes. Gallery previews show what was
checked, but the receipts are the proof.

## Parts

- `shaft-bearing-bracket`: bearing seat plus loaded M3 side mounts.
- `slotted-motor-plate`: adjustable straight slot plus counterbored fasteners.
- `edge-safe-slot-mount`: mechanical straight slot with enough material between
  the full slot envelope and the nearest free edge.
- `edge-safe-counterbore-mount`: mechanical counterbore with enough material
  between the full screw-head recess and the nearest free edge.
- `edge-safe-bearing-seat`: loaded 608 bearing seat with enough material around
  the full seat envelope.
- `edge-safe-insert-pocket`: mechanical M3 insert pocket with enough material
  around the full pocket envelope.
- `edge-safe-standoff-boss`: M3 standoff boss with enough material around the
  full boss envelope.
- `electronics-standoff-deck`: heat-set insert pockets plus clearance holes.
- `boss-supported-m3-mount`: boss-supported M3 mount with enough radial
  material around the fastener.
- `dense-random-hole-plate`: mechanical mount holes plus many cosmetic holes
  checked by a count rule.
- `relief-envelope-plate`: custom cosmetic relief cutout with an explicit
  spacing envelope and enough ligament to pass.
- `t-slot-linear-slider`: two-part captured slider with declared clearance
  windows and capture lips.
- `practical-insert-fit-good`: M3 insert pocket with a declared fit clearance
  and depth margin.
- `practical-driver-access-good`: service screw with a declared driver-access
  envelope.
- `practical-mount-pattern-good`: four-hole mount pattern with declared pitch
  consistency.
- `practical-snap-hook-good`: printable snap-hook pair with declared thickness
  and engagement.
- `practical-boss-support-good`: raised M3 boss with declared ratio and support
  ribs.
- actuator repair proof: before/after actuator mount receipts showing bad CAD,
  ordered repair guidance, and the fixed CAD pass.
- `dense-random-hole-plate-too-few-reliefs`: negative fixture for feature-count
  rules.
- `relief-envelope-plate-thin-ligament`: negative fixture for explicit
  spacing-envelope ligament rules.
- `t-slot-linear-slider-loose-clearance`: negative fixture for numeric clearance
  rules.
- `t-slot-linear-slider-tight-clearance`: negative fixture for captured-slider
  clearance that is too tight for the declared fit window.
- `t-slot-linear-slider-missing-capture-lip`: negative fixture proving a slider
  needs declared capture lips to avoid lift-off.
- `t-slot-linear-slider-shallow-capture-lip`: negative fixture proving capture
  lips need enough declared engagement.
- `dense-random-hole-plate-too-many-reliefs`: negative fixture proving cosmetic
  relief inventories can be bounded without treating every hole as mechanical.
- `hole-slot-thin-ligament`: negative fixture proving declared hole-to-slot
  ligaments can be checked from metadata.
- `bad-practical-insert-fit-tight`: negative fixture proving a visible insert
  pocket still needs a declared fit window.
- `bad-practical-driver-access-blocked`: negative fixture proving a screw hole
  also needs tool access.
- `bad-practical-mount-pattern-shifted`: negative fixture proving a four-hole
  pattern can fail by declared pitch error even when all holes exist.
- `bad-practical-snap-hook-thin`: negative fixture proving printable retention
  tabs need enough declared thickness.
- `bad-practical-boss-support-unsupported`: negative fixture proving a tall boss
  needs declared support ribs and a sane height-to-diameter ratio.
- `bad-slot-near-edge`: negative fixture proving Burr checks the whole slot
  envelope, not only a point at the slot center.
- `bad-counterbore-near-edge`: negative fixture proving Burr checks the larger
  counterbore recess envelope, not only the smaller through-hole.
- `bad-bearing-seat-near-edge`: negative fixture proving Burr checks loaded
  bearing seat edge material, not only whether the STEP seat exists.
- `bad-insert-pocket-near-edge`: negative fixture proving Burr checks the full
  insert pocket envelope against the host part edge.
- `bad-standoff-boss-near-edge`: negative fixture proving Burr checks the full
  boss footprint against the host part edge.
- `bad-boss-support-too-thin`: negative fixture for boss radial wall checks
  around a fastener.
- `bad-standoff-boss-missing-step`: negative fixture proving declared standoff
  bosses must exist in the exported STEP, not only in metadata.
