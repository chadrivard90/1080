#!/usr/bin/env bash
# Finish installing an extracted release (USB copy or download): checks libraries and
# creates the desktop icons. Run once after extracting race-dash-aarch64.tar.gz:
#
#   mkdir -p ~/race-dash
#   tar -xzf race-dash-aarch64.tar.gz -C ~/race-dash
#   bash ~/race-dash/scripts/setup.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ "$(uname -m)" != "aarch64" ]; then
    echo "!! Race Dash needs 64-bit Raspberry Pi OS (aarch64). This system is $(uname -m)."
    exit 1
fi
if [ ! -f "$DIR/race-dash" ]; then
    echo "!! $DIR/race-dash not found. Extract race-dash-aarch64.tar.gz (the release asset,"
    echo "   not GitHub's 'Source code' download) into $DIR first."
    exit 1
fi
chmod +x "$DIR/race-dash"

if ldd "$DIR/race-dash" | grep -q "not found"; then
    echo "!! Missing system libraries:"
    ldd "$DIR/race-dash" | grep "not found"
    echo "   Install them with: sudo apt-get install -y libfontconfig1 libxkbcommon0"
fi

bash "$DIR/scripts/make-shortcuts.sh"

echo
if [ -f "$DIR/REPO" ]; then
    echo "==> Updates will come from github.com/$(cat "$DIR/REPO") via the 'Update Race Dash' icon."
fi
echo "==> Installed build $(cut -c1-7 "$DIR/VERSION" 2>/dev/null || echo '?'). Start it with the 'Race Dash' icon."
