#!/usr/bin/env bash
# Local preview of the full published site: the landing/content site with the
# Bevy WASM game reachable at /play/, exactly like the GitHub Pages deploy but
# served from your machine. This is the only way to click "Play" locally - the
# webpack dev server (`npm run serve` in web/) does not know about the game and
# will just fall back to the landing page for /play/.
#
# Run it from inside the dev shell so both `trunk` and `node`/`npm` are present:
#
#     nix develop -c scripts/preview-web.sh            # debug game build (fast-ish)
#     nix develop -c scripts/preview-web.sh --release  # optimized game build
#
# Then open http://localhost:8090/ and click Play.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PORT="${PORT:-8090}"
TRUNK_ARGS=()
if [[ "${1:-}" == "--release" ]]; then
    TRUNK_ARGS+=(--release)
fi

echo ">> building the game (trunk)…"
# Relative public_url (Trunk.toml) keeps the game position-independent, so the
# same build works under /play/. Output goes to ./dist by default.
trunk build "${TRUNK_ARGS[@]}"

echo ">> building the landing site (webpack)…"
pushd web >/dev/null
if [[ ! -d node_modules ]]; then
    npm install
fi
# Default PUBLIC_PATH (/) so the preview is served from the root locally.
npm run build
popd >/dev/null

echo ">> assembling combined preview…"
# webpack's clean wiped web/dist, so drop the game in afterwards.
rm -rf web/dist/play
mkdir -p web/dist/play
cp -r dist/. web/dist/play/

echo ">> serving http://localhost:${PORT}/  (Ctrl-C to stop)"
npx --yes http-server web/dist -p "${PORT}" -c-1
