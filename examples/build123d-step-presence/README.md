# STEP Presence Fixtures

These fixtures prove declared-intent checking.

- `good/` declares `m3_claimed` and creates matching STEP cylinder geometry.
- `bad/` declares `m3_claimed` but intentionally skips geometry with `create_geometry=False`.

Burr fails `bad/` because a declared feature is missing, not because it scans the
STEP and decides which holes matter. Extra cylinders in a STEP file should stay
undeclared unless a rule should judge them.
