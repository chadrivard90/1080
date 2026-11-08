#!/usr/bin/env bash
# Download the open-licence Google Fonts the dashes use into ./fonts (needed before building).
# All are SIL Open Font License, except Permanent Marker (Apache 2.0).
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$REPO_DIR/fonts"
BASE="https://raw.githubusercontent.com/google/fonts/main"
mkdir -p "$DEST"

fetch() {
    local path="$1" file
    file="$(basename "$path")"
    if [ -s "$DEST/$file" ]; then
        return
    fi
    echo "==> Downloading font $file"
    curl -fsSL --retry 3 "$BASE/$path" -o "$DEST/$file"
}

fetch ofl/titilliumweb/TitilliumWeb-Bold.ttf
fetch ofl/titilliumweb/TitilliumWeb-Black.ttf
fetch ofl/sharetechmono/ShareTechMono-Regular.ttf
fetch ofl/russoone/RussoOne-Regular.ttf
fetch ofl/anton/Anton-Regular.ttf
fetch ofl/bebasneue/BebasNeue-Regular.ttf
fetch apache/permanentmarker/PermanentMarker-Regular.ttf
