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
