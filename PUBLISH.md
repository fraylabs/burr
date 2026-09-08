# Releasing Burr

Burr is currently distributed from its public Git repository. The npm manifest
is private and exists only as a local task runner. Cargo publishing is disabled
because the pinned Look/Truck stack is not registry-publishable.

Before creating a version tag:

```bash
npm run check
cargo build --release --locked
```

Keep `Cargo.toml`, `package.json`, `Cargo.lock`, and `CHANGELOG.md` on the same
version. Tag the clean checked commit using the existing
`burr-v<version>` convention.

Users can install a tagged release directly from Git:

```bash
cargo install --git https://github.com/fraylabs/burr.git --tag burr-v0.34.0 --locked
```

Do not enable crates.io publishing until Look and its required Truck forks have
registry-compatible releases. Do not publish the private npm task manifest.
