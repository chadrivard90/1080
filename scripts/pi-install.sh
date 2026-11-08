#!/usr/bin/env bash
# Install Race Dash on the Pi from the latest GitHub release. No compiling needed.
#
# Usage (on the Pi):
#   curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/scripts/pi-install.sh | bash -s OWNER/REPO
set -euo pipefail

REPO="${1:-}"
if [ -z "$REPO" ]; then
    echo "Usage: pi-install.sh OWNER/REPO   (your GitHub user / repository name)"
    exit 1
fi
INSTALL_DIR="${RACE_DASH_DIR:-$HOME/race-dash}"

if [ "$(uname -m)" != "aarch64" ]; then
    echo "!! This build needs 64-bit Raspberry Pi OS (aarch64). This system is $(uname -m)."
    exit 1
fi

URL="https://github.com/$REPO/releases/latest/download/race-dash-aarch64.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "==> Downloading $URL"
curl -fL --progress-bar "$URL" -o "$tmp/race-dash.tar.gz"

pkill -x race-dash >/dev/null 2>&1 || true
mkdir -p "$INSTALL_DIR"
tar -xzf "$tmp/race-dash.tar.gz" -C "$INSTALL_DIR"
echo "$REPO" >"$INSTALL_DIR/REPO"

if ldd "$INSTALL_DIR/race-dash" | grep -q "not found"; then
    echo "!! Missing system libraries:"
    ldd "$INSTALL_DIR/race-dash" | grep "not found"
    echo "   Try: sudo apt-get install -y libfontconfig1 libxkbcommon0"
fi

bash "$INSTALL_DIR/scripts/make-shortcuts.sh"

echo
echo "==> Race Dash installed in $INSTALL_DIR (build $(cut -c1-7 "$INSTALL_DIR/VERSION"))"
echo "    Start it with the 'Race Dash' desktop icon; update with 'Update Race Dash'."
