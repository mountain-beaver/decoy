#!/usr/bin/env bash
# Linux only: build decoy-daemon and put it where Tauri expects the sidecar
# (src-tauri/binaries/decoy-daemon-<target triple>). Run it before building or
# testing the `decoy` crate; `cargo tauri build` runs it through
# src-tauri/tauri.linux.conf.json.
#
#   scripts/prepare-daemon.sh            release build (what bundles ship)
#   scripts/prepare-daemon.sh --debug    debug build (fast, for tests and clippy)
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

profile=release
cargo_flags=(--release)
if [[ "${1:-}" == "--debug" ]]; then
  profile=debug
  cargo_flags=()
fi

# The Tauri CLI exports the target triple when it runs beforeBuildCommand.
target="${TAURI_ENV_TARGET_TRIPLE:-}"
if [[ -n "$target" ]]; then
  cargo build "${cargo_flags[@]}" -p decoy-daemon --target "$target"
  built="${CARGO_TARGET_DIR:-target}/$target/$profile/decoy-daemon"
else
  target="$(rustc -vV | sed -n 's/^host: //p')"
  cargo build "${cargo_flags[@]}" -p decoy-daemon
  built="${CARGO_TARGET_DIR:-target}/$profile/decoy-daemon"
fi

mkdir -p src-tauri/binaries
install -m 755 "$built" "src-tauri/binaries/decoy-daemon-$target"
echo "prepared src-tauri/binaries/decoy-daemon-$target ($profile)"
