# burr-ocp

Optional OpenCascade helpers for Burr.

`burr-ocp-step-cylinders` loads a STEP file with OCP/OpenCascade and prints
measured cylindrical faces as JSON. Burr can use this as an optional geometry
backend while keeping the Rust CLI installable without OpenCascade.

```bash
uv run --package burr-ocp burr-ocp-step-cylinders part.step
```
