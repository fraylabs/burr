#!/bin/sh
# Install a prebuilt Burr. Optional: BURR_VERSION=0.35.0, BURR_INSTALL_DIR=/path.
set -eu

die() { printf 'burr: %s\n' "$*" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || die 'curl is required'
command -v tar >/dev/null 2>&1 || die 'tar is required'

case "$(uname -s)" in
    Darwin) os=apple-darwin ;;
    Linux) os=unknown-linux-gnu ;;
    *) die 'supported systems: macOS and Linux' ;;
esac
case "$(uname -m)" in
    arm64|aarch64) arch=aarch64 ;;
    x86_64|amd64) arch=x86_64 ;;
    *) die 'unsupported CPU architecture' ;;
esac
# Prefer the native Apple Silicon binary when invoked from a Rosetta shell.
if [ "$os" = apple-darwin ] && [ "$(/usr/sbin/sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ]; then
    arch=aarch64
fi
[ "$os/$arch" != unknown-linux-gnu/aarch64 ] || die 'Linux arm64 binaries are not available'
if command -v shasum >/dev/null 2>&1; then
    checksum() { shasum -a 256 -c "$1"; }
elif command -v sha256sum >/dev/null 2>&1; then
    checksum() { sha256sum -c "$1"; }
else
    die 'shasum or sha256sum is required'
fi

base=https://github.com/fraylabs/burr/releases
version=${BURR_VERSION:-latest}
if [ "$version" = latest ]; then
    url=$base/latest/download
else
    version=${version#burr-v}
    version=${version#v}
    case "$version" in ''|*[!0-9A-Za-z.+-]*) die 'invalid BURR_VERSION' ;; esac
    url=$base/download/burr-v$version
fi
archive=burr-$arch-$os.tar.gz
tmp=$(mktemp -d)
staged=
trap 'rm -rf "$tmp"; [ -z "$staged" ] || rm -f "$staged"' 0
trap 'exit 1' HUP INT TERM
printf 'Downloading Burr (%s-%s)...\n' "$arch" "$os"
curl -fsSL --retry 2 --connect-timeout 15 "$url/$archive" -o "$tmp/$archive" || die 'download failed; check that this release has prebuilt binaries'
curl -fsSL --retry 2 --connect-timeout 15 "$url/$archive.sha256" -o "$tmp/$archive.sha256" || die 'checksum download failed'
(cd "$tmp" && checksum "$archive.sha256") || die 'checksum verification failed'
tar -xzf "$tmp/$archive" -C "$tmp" burr
installed_version=$("$tmp/burr" --version) || die 'downloaded binary cannot run on this system'
dir=${BURR_INSTALL_DIR:-$HOME/.local/bin}
mkdir -p "$dir"
staged=$(mktemp "$dir/.burr.XXXXXX")
install -m 755 "$tmp/burr" "$staged"
mv -f "$staged" "$dir/burr"
staged=
printf 'Installed Burr %s to %s/burr\n' "$installed_version" "$dir"
# Print a literal $PATH for the user to paste into their shell profile.
# shellcheck disable=SC2016
case ":$PATH:" in
    *":$dir:"*) ;;
    *) printf 'Add this directory to PATH (and your shell profile):\n  export PATH="%s:$PATH"\n' "$dir" ;;
esac
