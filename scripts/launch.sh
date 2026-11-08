#!/usr/bin/env bash
# Start Race Dash fullscreen. Used by the desktop icon.
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Release installs ship the binary at the top level; source installs build into target/.
BIN="$REPO_DIR/race-dash"
if [ ! -x "$BIN" ]; then
    BIN="$REPO_DIR/target/release/race-dash"
fi
LOG="${XDG_CACHE_HOME:-$HOME/.cache}/race-dash.log"
mkdir -p "$(dirname "$LOG")"

if [ ! -x "$BIN" ]; then
    echo "Race Dash is not built yet. Run the 'Update Race Dash' icon (or scripts/build.sh) first." | tee -a "$LOG"
    exit 1
fi

# The UI is laid out in physical pixels for the 1080x1920 panel.
export SLINT_SCALE_FACTOR="${SLINT_SCALE_FACTOR:-1}"
export SLINT_FULLSCREEN="${SLINT_FULLSCREEN:-1}"

# Only one instance at a time.
if pgrep -x race-dash >/dev/null 2>&1; then
    exit 0
fi

exec "$BIN" >>"$LOG" 2>&1
