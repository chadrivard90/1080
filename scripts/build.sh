#!/usr/bin/env bash
# Build the release binary. The first build on a Pi takes a while (Slint is big);
# later builds only recompile what changed.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi
cd "$REPO_DIR"

JOBS=()
mem_kb=$(awk '/MemTotal/ {print $2}' /proc/meminfo 2>/dev/null || echo 8000000)
if [ "$mem_kb" -lt 3000000 ]; then
    # Pis with <= 2 GB RAM run out of memory with too many parallel rustc jobs.
    JOBS=(-j 2)
fi

bash "$REPO_DIR/scripts/fetch-fonts.sh"
echo "==> Building Race Dash (release)"
cargo build --release "${JOBS[@]}"
echo "==> Build finished: $REPO_DIR/target/release/race-dash"
