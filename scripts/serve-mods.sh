#!/usr/bin/env bash
# Build the static mod portal and keep it fresh while you edit `webmods/`.
#
# The portal is a generated static tree (scripts/gen-portal.py), so unlike the
# game and the site it has no watch mode of its own. This wraps it in one:
# generate once, serve on a random 7XXX port, then regenerate on every change
# under `webmods/` (or to the shipped catalog) via watchexec. Reload the Explore
# tab and your edit is there - no restart.
#
# Run it from inside the dev shell (watchexec and python3 both come from there):
#
#     nix develop -c scripts/serve-mods.sh              # serve + watch
#     nix develop -c scripts/serve-mods.sh --once       # generate and exit
#     NOVA_MODS_PORT=9000 nix develop -c scripts/serve-mods.sh
#
# `scripts/serve-web.sh` starts this for you as part of the full preview; run it
# directly for the game-only loop (see Trunk.toml's [[proxy]], which expects the
# portal on :9000) or to eyeball the generated tree.
#
# The wasm game fetches the portal SAME-ORIGIN (it derives the base from
# window.location), so it never talks to this port directly - trunk or the
# webpack dev server proxies to it. Native builds can point straight at it with
# NOVA_PORTAL_URL. See web/src/wiki/dev/mod-portal.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
# shellcheck source=scripts/dev-ports.sh
source "$ROOT/scripts/dev-ports.sh"

ONCE=0
if [[ "${1:-}" == "--once" ]]; then
    ONCE=1
elif [[ $# -gt 0 ]]; then
    echo "usage: $0 [--once]" >&2
    exit 2
fi

# Served root, NOT the portal itself: gen-portal.py writes into <root>/mods so
# the served layout matches the deploy (portal at /mods/). Under target/ it is
# already gitignored and wiped by `cargo clean`.
SERVE_ROOT="${NOVA_MODS_DIR:-$ROOT/target/portal-preview}"
OUT="$SERVE_ROOT/mods"

generate() {
    # Full rebuild: gen-portal.py merges into an existing directory, so a hashed
    # copy left behind by a deleted or renamed mod would keep being served even
    # though the fresh catalog.json no longer lists it.
    rm -rf "$OUT"
    mkdir -p "$SERVE_ROOT"
    python3 scripts/gen-portal.py \
        --source webmods \
        --shipped assets/mods.catalog.ron \
        --out "$OUT"
}

echo ">> generating the mod portal into $OUT"
generate

if ((ONCE)); then
    exit 0
fi

PORT="$(nova_resolve_port NOVA_MODS_PORT)"

PIDS=()
cleanup() {
    trap - EXIT INT TERM
    # Kill by recorded PID only - a pattern kill here would take out the other
    # preview servers, and any unrelated python http.server on the machine.
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# ThreadingHTTPServer since 3.7, so a stalled asset fetch cannot block the
# catalog request behind it.
python3 -m http.server --bind 127.0.0.1 --directory "$SERVE_ROOT" "$PORT" >/dev/null 2>&1 &
PIDS+=("$!")

if ! nova_wait_for_port "$PORT" 10; then
    echo "!! the portal server did not come up on :$PORT" >&2
    exit 1
fi

echo ">> portal: http://localhost:${PORT}/mods/catalog.json  (Ctrl-C to stop)"
echo ">> watching webmods/ - edits regenerate the portal in place"

# --postpone: we already generated above, so only react to real edits.
# assets/mods.catalog.ron is watched too because it is the collision gate - a new
# shipped id can invalidate a portal id.
watchexec \
    --postpone \
    --debounce 500ms \
    --watch webmods \
    --watch assets/mods.catalog.ron \
    -- "$ROOT/scripts/serve-mods.sh" --once &
PIDS+=("$!")

wait -n
