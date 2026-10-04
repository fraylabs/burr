# Releasing Burr

Burr ships prebuilt binaries on GitHub releases. The npm manifest is private
and exists only as a local task runner. Cargo publishing stays disabled because
the pinned Look/Truck stack is not registry-publishable.

Before creating a version tag:

```bash
npm run check
cargo build --release --locked
```

Keep `Cargo.toml`, `package.json`, `Cargo.lock`, and `CHANGELOG.md` on the same
version. Tag the clean checked commit using the existing `burr-v<version>`
convention. The release workflow checks that the tag matches `Cargo.toml`.

## Binary releases

Pushing a `burr-v*` tag runs `.github/workflows/release.yml`. Its shared build
workflow uses standard GitHub-hosted runners (free for this public repository):

| Target | Runner | Minimum system |
| --- | --- | --- |
| `aarch64-apple-darwin` | `macos-14` | macOS 11 |
| `x86_64-apple-darwin` | `macos-15-intel` | macOS 11, AVX/FMA CPU |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | glibc 2.35, AVX/FMA CPU |

Builds use `cargo build --release --locked` with the pinned Git dependencies.
The repository's `.cargo/config.toml` enables AVX/FMA only on x86-64.
`MACOSX_DEPLOYMENT_TARGET=11.0` sets the macOS compatibility floor.
`LZMA_API_STATIC=1` builds liblzma into the binaries instead of linking to the
runner's Homebrew xz. Packaging rejects Mac binaries that link to anything
outside `/usr/lib` or `/System/Library`.

Each target produces `burr-<target>.tar.gz` (the binary and MIT license) and
`burr-<target>.tar.gz.sha256`. After all three succeed, the workflow creates the
GitHub release if needed and uploads those six files plus `install.sh`. Reruns
replace the assets for the same tag. No signing, notarization, external service,
or repository secret is needed; the release job uses GitHub's built-in token.

Manual runs only upload Actions artifacts, retained for 14 days. They never
create or modify a GitHub release, even when dispatched against a tag:

```bash
gh workflow run release.yml --ref your-branch
```

Before the release workflow exists on the default branch, use the existing
Check workflow's manual entry point. It runs checks and the same binary builds:

```bash
gh workflow run check.yml --ref your-branch
gh run list --branch your-branch
gh run download RUN_ID --name burr-aarch64-apple-darwin --dir /tmp/burr-binary
```

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/fraylabs/burr/main/install.sh | sh
```

The POSIX installer selects the host target, downloads the latest release's
binary and checksum, verifies the checksum, and installs to `~/.local/bin`.
Set `BURR_VERSION=0.36.0` (also accepts `v0.36.0` or `burr-v0.36.0`) to pin a
release, and `BURR_INSTALL_DIR` to change the destination. It prints a PATH
instruction if necessary. No Rust toolchain is used.

The installer requires a release containing the new binary assets. Existing
source-only tags predate this workflow, so a maintainer must manually upload
validated binaries built from the matching version to backfill those releases.
Alternatively, publish the next version with a new tag after merging the
workflow. Until the workflow and script are merged and a binary release is
published, the new install command is not available on `main`.

Users can still install from source:

```bash
cargo install --git https://github.com/fraylabs/burr.git --tag burr-v0.36.0 --locked
```

On x86-64, prefix the command with `RUSTFLAGS="-Ctarget-feature=+avx,+fma"`.
Do not enable crates.io publishing until Look and its required Truck forks have
registry-compatible releases. Do not publish the private npm task manifest.
