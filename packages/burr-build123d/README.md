# burr-build123d

build123d helpers that emit Burr design data.

Install in a build123d project:

```bash
uv add burr-build123d
```

Install the Burr CLI separately:

```bash
cargo install burr
```

```python
from burr_build123d import BurrDesignData, m3_clearance_hole
```

This package does not replace build123d. It records mechanical intent while
normal build123d code creates geometry.

For boss-supported fasteners, pass `support_diameter_mm` to the helper so Burr
can check the declared radial material around the hole or insert pocket.

Use `standoff_boss(...)` when the raised boss itself should be checked in the
exported STEP. Burr verifies the declared boss cylinder and top face separately
from the hole or insert it supports.

When that boss supports a declared hole or insert, pass
`supports_feature_id="..."`. Burr can then check that the boss centerline, axis,
and declared support diameter align with the feature it claims to support.

For custom reliefs or cutouts, keep creating geometry in normal build123d code
and attach an explicit spacing envelope to the declared feature:

```python
from burr_build123d import spacing_envelope

design.feature(
    feature_id="rounded_relief_window",
    kind="cutout",
    part="plate",
    intent="cosmetic",
    role="relief_slot",
    spacing_envelope=spacing_envelope(
        segment_start=(0, -8, 0),
        segment_end=(0, 8, 0),
        radius_mm=3.0,
    ),
)
```

`spacing_envelope(...)` emits metadata only. It does not create or verify the
cutout geometry by itself.
