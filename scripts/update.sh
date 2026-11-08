#!/usr/bin/env bash
# "Update Race Dash" desktop icon (runs in a terminal window).
#
# Release install (from pi-install.sh): downloads the latest prebuilt release from GitHub.
# Source install (git clone + install-from-source.sh): git pull + rebuild on the Pi.
#
# Everything lives in main() so bash has read the whole script before an update
# overwrites this file.
set -uo pipefail

main() {
    local DIR
    DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    cd "$DIR" || exit 1

    echo "=============================="
    echo "   RACE DASH UPDATER"
    echo "=============================="
    echo

    if [ -f "$DIR/REPO" ]; then
        update_from_release "$DIR" || pause_exit 1
    else
        update_from_source "$DIR" || pause_exit 1
    fi

    bash "$DIR/scripts/make-shortcuts.sh" >/dev/null 2>&1 || true

    echo
    local answer
    read -r -t 20 -p "Launch Race Dash now? [Y/n] " answer || answer="y"
    case "${answer:-y}" in
        [nN]*) pause_exit 0 ;;
        *) nohup bash "$DIR/scripts/launch.sh" >/dev/null 2>&1 & disown; sleep 1 ;;
    esac
    exit 0
}

pause_exit() {
    echo
    read -r -t 60 -p "Press Enter to close..." _ || true
    exit "${1:-0}"
}

stop_running() {
    if pgrep -x race-dash >/dev/null 2>&1; then
        echo "==> Closing running Race Dash"
        pkill -x race-dash || true
        sleep 1
    fi
}

update_from_release() {
    local dir="$1" repo current latest tmp
    repo="$(cat "$dir/REPO")"
    current="$(cat "$dir/VERSION" 2>/dev/null || echo none)"
    tmp="$(mktemp -d)"

    echo "==> Checking github.com/$repo for the latest build"
    if ! curl -fL --progress-bar "https://github.com/$repo/releases/latest/download/race-dash-aarch64.tar.gz" \
        -o "$tmp/race-dash.tar.gz"; then
        echo "!! Download failed. Check the network, and that the repo is public with at least one release."
        rm -rf "$tmp"
        return 1
    fi
    mkdir -p "$tmp/new"
    tar -xzf "$tmp/race-dash.tar.gz" -C "$tmp/new" || { rm -rf "$tmp"; return 1; }
    latest="$(cat "$tmp/new/VERSION" 2>/dev/null || echo unknown)"

    if [ "$latest" = "$current" ]; then
        echo "==> Already up to date (build ${current:0:7})"
    else
        stop_running
        cp -a "$tmp/new/." "$dir/"
        echo "$repo" >"$dir/REPO"
        echo "==> Updated ${current:0:7} -> ${latest:0:7}"
    fi
    rm -rf "$tmp"
}

update_from_source() {
    local dir="$1" branch before after
    branch="$(git -C "$dir" rev-parse --abbrev-ref HEAD)"
    before="$(git -C "$dir" rev-parse HEAD)"
    echo "==> Fetching updates ($branch)"
    git -C "$dir" fetch --prune origin || { echo "!! Could not reach GitHub."; return 1; }
    if ! git -C "$dir" merge --ff-only "origin/$branch"; then
        echo "!! Update could not be applied automatically (local changes on the Pi?)."
        echo "   To match GitHub exactly: cd \"$dir\" && git reset --hard origin/$branch"
        return 1
    fi
    after="$(git -C "$dir" rev-parse HEAD)"
    if [ "$before" = "$after" ] && [ -x "$dir/target/release/race-dash" ]; then
        echo "==> Already up to date ($(git -C "$dir" log -1 --format='%h %s'))"
    else
        stop_running
        echo "==> Updated to: $(git -C "$dir" log -1 --format='%h %s')"
        bash "$dir/scripts/build.sh" || { echo "!! Build failed."; return 1; }
    fi
}

main "$@"
