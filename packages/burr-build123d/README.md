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
