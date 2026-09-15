#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Boot CrownOS in a VM.
#
# Nested (`CROWN_BACKEND=winit`) is the fast loop, but it never exercises the
# code that decides whether CrownOS works on real hardware: seat acquisition,
# DRM/KMS mode setting, and the session launcher a display manager would use.
# This does, and it cannot break your machine.
#
#   ./contrib/run-vm.sh                 # build, then boot
#   ./contrib/run-vm.sh --no-build      # boot what is already built
#   ./contrib/run-vm.sh --release       # use the release binaries
#
# The guest mounts this repo's target directory read-only at /crownos and
# autologins into crownos-session. Nothing is installed into the guest image, so
# a rebuild on the host is picked up by the next boot.

set -euo pipefail

REPO="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SETUP_FLAKE="${CROWNOS_SETUP_FLAKE:-github:Crown-OS/crownOs-setup}"
PROFILE=debug
BUILD=1

die()  { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
note() { printf '\033[36m==>\033[0m %s\n' "$*"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build) BUILD=0 ;;
    --release)  PROFILE=release ;;
    -h|--help)  sed -n '3,18p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'; exit 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
  shift
done

command -v nix >/dev/null || die "nix is required to build the VM image.
  Install it, or use the nested session instead:  CROWN_BACKEND=winit crownos-session"

[[ -e /dev/kvm ]] || cat >&2 <<'EOF'
warning: /dev/kvm is not available, so the VM will run under emulation.
  It will boot, but slowly. On most systems you need to be in the `kvm` group.
EOF

if [[ $BUILD -eq 1 ]]; then
  note "building the workspace ($PROFILE)"
  if [[ $PROFILE == release ]]; then
    ( cd "$REPO" && cargo build --workspace --release )
  else
    ( cd "$REPO" && cargo build --workspace )
  fi
fi

BIN="$REPO/target/$PROFILE"
for b in crownpositor crownbar crowndock crownotify; do
  [[ -x "$BIN/$b" ]] || die "$BIN/$b is missing. Run without --no-build, or build first."
done

# crownos-session is a script, not a cargo target, so it is not in target/.
# Stage it next to the binaries so the guest finds one directory with everything.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp "$BIN"/crownpositor "$BIN"/crownbar "$BIN"/crowndock "$BIN"/crownotify "$STAGE/"
[[ -x "$BIN/crowndictator" ]] && cp "$BIN/crowndictator" "$STAGE/"
install -m755 "$REPO/session/crownos-session" "$STAGE/crownos-session"

note "staged $(ls "$STAGE" | wc -l) binaries at $STAGE"
note "building the VM image (first run downloads a NixOS closure; later runs are cached)"

# The VM's run script is named after the guest hostname. Glob for it rather than
# hard-coding, so renaming the host in vm.nix cannot break this.
# --refresh because nix caches the resolved revision of a github: flake ref.
# Without it, the first run after someone pushes a change to crownOs-setup
# fails with "does not provide attribute", which reads like a broken flake
# rather than a stale cache.
VM_OUT="$(nix build --refresh --no-link --print-out-paths \
  "${SETUP_FLAKE}#nixosConfigurations.crownos-vm.config.system.build.vm")"
# No -type f: nixpkgs ships this as a symlink into the store, and -type f
# silently excludes symlinks, which makes the glob find nothing.
RUNNER="$(find -L "$VM_OUT/bin" -name 'run-*-vm' | head -1)"
[[ -n "$RUNNER" ]] || die "no run-*-vm script in $VM_OUT/bin"

note "booting — log in happens automatically; Super+Shift+E quits the session"
echo
CROWNOS_BIN="$STAGE" exec "$RUNNER"
