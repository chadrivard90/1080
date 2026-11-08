#!/usr/bin/env bash
# Alternative to pi-install.sh: build Race Dash on the Pi itself from a git clone.
#   Installs build dependencies + Rust, builds Race Dash, creates the desktop icons.
# Usage:  bash scripts/install-from-source.sh
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Installing system packages (needs sudo)"
sudo apt-get update
sudo apt-get install -y build-essential pkg-config git curl libfontconfig1-dev libxkbcommon-dev
# Optional extras used by some Slint backends; ignore any that don't exist on this OS release.
for pkg in libfreetype-dev libudev-dev libinput-dev libgbm-dev libdrm-dev libxkbcommon-x11-dev fonts-dejavu-core; do
    sudo apt-get install -y "$pkg" >/dev/null 2>&1 || echo "    (skipped optional package $pkg)"
done

if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "==> Installing Rust (rustup)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

bash "$REPO_DIR/scripts/build.sh"
bash "$REPO_DIR/scripts/make-shortcuts.sh"

echo
echo "==> Done. Use the 'Race Dash' icon on the desktop to start,"
echo "    and 'Update Race Dash' to pull the latest version from GitHub."
