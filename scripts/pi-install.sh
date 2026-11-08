#!/usr/bin/env bash
# Install Race Dash on the Pi from the latest GitHub release (alternative to copying the
# tar.gz over USB). No compiling needed.
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

URL="https://github.com/$REPO/releases/latest/download/race-dash-aarch64.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "==> Downloading $URL"
curl -fL --progress-bar "$URL" -o "$tmp/race-dash.tar.gz"

pkill -x race-dash >/dev/null 2>&1 || true
mkdir -p "$INSTALL_DIR"
tar -xzf "$tmp/race-dash.tar.gz" -C "$INSTALL_DIR"
echo "$REPO" >"$INSTALL_DIR/REPO"

bash "$INSTALL_DIR/scripts/setup.sh"
