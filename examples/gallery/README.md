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
- `electronics-standoff-deck`: heat-set insert pockets plus clearance holes.
- `dense-random-hole-plate`: mechanical mount holes plus many cosmetic holes
  checked by a count rule.
- `relief-envelope-plate`: custom cosmetic relief cutout with an explicit
  spacing envelope and enough ligament to pass.
- `t-slot-linear-slider`: two-part captured slider with declared clearance
  windows and capture lips.
- actuator repair proof: before/after actuator mount receipts showing bad CAD,
  ordered repair guidance, and the fixed CAD pass.
- `dense-random-hole-plate-too-few-reliefs`: negative fixture for feature-count
  rules.
- `relief-envelope-plate-thin-ligament`: negative fixture for explicit
  spacing-envelope ligament rules.
- `t-slot-linear-slider-loose-clearance`: negative fixture for numeric clearance
  rules.
