#!/usr/bin/env bash
# Create the "Race Dash" and "Update Race Dash" desktop icons (and app-menu entries).
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")"
APPS_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR" "$APPS_DIR"

write_entry() {
    local file="$1" name="$2" comment="$3" exec_cmd="$4" icon="$5" terminal="$6"
    cat >"$file" <<EOF
[Desktop Entry]
Type=Application
Name=$name
Comment=$comment
Exec=$exec_cmd
Icon=$icon
Terminal=$terminal
Categories=Game;Utility;
EOF
    chmod +x "$file"
    # Mark as trusted where the file manager supports it (avoids the "untrusted launcher" prompt).
    gio set "$file" metadata::trusted true >/dev/null 2>&1 || true
}

APP_ICON="$REPO_DIR/assets/race-dash.svg"
UPDATE_ICON="$REPO_DIR/assets/race-dash-update.svg"
[ -f "$REPO_DIR/art/icon-app.png" ] && APP_ICON="$REPO_DIR/art/icon-app.png"
[ -f "$REPO_DIR/art/icon-update.png" ] && UPDATE_ICON="$REPO_DIR/art/icon-update.png"

for dir in "$DESKTOP_DIR" "$APPS_DIR"; do
    write_entry "$dir/race-dash.desktop" "Race Dash" "Sim racing telemetry dashboard" \
        "bash \"$REPO_DIR/scripts/launch.sh\"" "$APP_ICON" false
    write_entry "$dir/race-dash-update.desktop" "Update Race Dash" "Download the latest Race Dash from GitHub and rebuild" \
        "bash \"$REPO_DIR/scripts/update.sh\"" "$UPDATE_ICON" true
done

# Raspberry Pi OS desktop (pcmanfm): launch desktop icons directly instead of asking
# "Execute / Execute in terminal / Open" every time.
LIBFM_CONF="$HOME/.config/libfm/libfm.conf"
mkdir -p "$(dirname "$LIBFM_CONF")"
if [ ! -f "$LIBFM_CONF" ]; then
    printf '[config]\nquick_exec=1\n' >"$LIBFM_CONF"
elif grep -q '^quick_exec=' "$LIBFM_CONF"; then
    sed -i 's/^quick_exec=.*/quick_exec=1/' "$LIBFM_CONF"
elif grep -q '^\[config\]' "$LIBFM_CONF"; then
    sed -i '/^\[config\]/a quick_exec=1' "$LIBFM_CONF"
else
    printf '\n[config]\nquick_exec=1\n' >>"$LIBFM_CONF"
fi
# The desktop only reads libfm.conf at startup; ask the running file manager to reload it.
if pgrep -x pcmanfm >/dev/null 2>&1; then
    pcmanfm --reconfigure >/dev/null 2>&1 || true
fi

echo "==> Desktop icons created in $DESKTOP_DIR"
