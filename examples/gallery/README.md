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

## Parts

- `shaft-bearing-bracket`: bearing seat plus loaded M3 side mounts.
- `slotted-motor-plate`: adjustable straight slot plus counterbored fasteners.
- `electronics-standoff-deck`: heat-set insert pockets plus clearance holes.
- `dense-random-hole-plate`: mechanical mount holes plus many cosmetic holes
  checked by a count rule.
- `t-slot-linear-slider`: two-part captured slider with declared clearance
  windows and capture lips.
