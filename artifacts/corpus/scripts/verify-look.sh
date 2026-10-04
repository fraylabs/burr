#!/bin/bash
# Run from the pinned Look checkout under the shared Burr build lock.
set -e
cargo fmt --all -- --check
cargo test --release --locked -p truck-geometry --test hyperbola
cargo test --release --locked -p truck-stepio --lib
cargo test --release --locked -p truck-stepio --test input -- --skip assy::occt_assy --skip tessellate_shape --skip table::read
cargo test --release --locked -p truck-stepio --test real_step_faces
cargo test --release --locked -p truck-meshalgo --test import_resilience
cargo test --release --locked -p truck-stepio --test occt_high_roi_cluster_001
cargo test --release --locked -p look --test assembly --test step
cargo test --release --locked -p look --test gpu_smoke -- --ignored --nocapture --test-threads=1
