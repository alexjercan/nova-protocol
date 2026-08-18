#!/usr/bin/env bash
# Live web development: the whole published site, watched and hot-reloading.
#
# Starts the three servers the deployed site is assembled from and wires them
# into one origin, so what you click locally has the same shape as GitHub Pages:
#
#     site   webpack dev server   /        <- watches web/src, rebuilds + reloads
#     game   trunk serve          /play/   <- watches the Rust/asset tree
#     portal serve-mods.sh        /mods/   <- watches webmods/
#
# The site's dev server proxies /play and /mods to the other two, which is what
# keeps the portal same-origin with the game (the wasm build derives its portal
# base from window.location and a cross-origin one dies on CORS).
#
# Each server gets a random free port in 7000-7999, so several worktrees can run
# this at once. Pin any of them with NOVA_UI_PORT / NOVA_GAME_PORT /
# NOVA_MODS_PORT. Run it from inside the dev shell:
#
#     nix develop -c scripts/serve-web.sh                   # debug game build
#     nix develop -c scripts/serve-web.sh --release         # optimized game build
#     nix develop -c scripts/serve-web.sh --open            # open the site when ready
#     nix develop -c scripts/serve-web.sh --release --open  # both, in any order
#
# The banner prints the URL to open; everything rebuilds on save until Ctrl-C.
#
# For a NON-watching preview of the real deploy layout (static files, no
# proxies, production-shaped paths) use scripts/preview-web.sh instead. That is
# the one to trust before a release; this one is for iterating.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
# shellcheck source=scripts/dev-ports.sh
source "$ROOT/scripts/dev-ports.sh"

RELEASE=0
OPEN_BROWSER=0
for arg in "$@"; do
    case "$arg" in
        --release) RELEASE=1 ;;
        --open) OPEN_BROWSER=1 ;;
        *)
            echo "usage: $0 [--release] [--open]" >&2
            exit 2
            ;;
    esac
done

TRUNK_ARGS=()
if ((RELEASE)); then
    TRUNK_ARGS+=(--release)
fi

# Explicit watch paths. Trunk's default is "the build target's parent folder",
# i.e. the repo root - and in this repo that watcher silently never fires, so
# `trunk serve` would serve the first build forever and you would chase ghosts.
# Naming the real build inputs makes it fire, and keeps the watcher off `target/`
# and `.git`. These are everything index.html pulls in: the workspace crates,
# the root binary, the copied asset dirs, and the manifests.
TRUNK_ARGS+=(
    --watch crates
    --watch src
    --watch assets
    --watch credits
    --watch build
    --watch index.html
    --watch Cargo.toml
    --watch Cargo.lock
)

UI_PORT="$(nova_resolve_port NOVA_UI_PORT)"
GAME_PORT="$(nova_resolve_port NOVA_GAME_PORT)"
MODS_PORT="$(nova_resolve_port NOVA_MODS_PORT)"

# Ports are allocated HERE, once, and pushed down - each server has to know
# where the others landed, and none of them can discover that on its own.
export NOVA_UI_PORT="$UI_PORT" # read by web/webpack.config.js
export NOVA_MODS_PORT="$MODS_PORT" # read by scripts/serve-mods.sh
export TRUNK_SERVE_PORT="$GAME_PORT" # trunk's own env override for [serve] port
# Trunk's dev-only backend proxy. Trunk.toml deliberately carries no [[proxy]]
# entry - Trunk merges a config-file proxy with this one and then panics on the
# duplicate /mods route. It only matters when you open the game directly on its
# own port (no /play prefix, so the portal base resolves to <game origin>/mods);
# through the site it is the webpack proxy below that serves /mods.
export TRUNK_SERVE_PROXY_BACKEND="http://127.0.0.1:${MODS_PORT}/mods"
# Proxy targets for the site dev server (web/webpack.config.js).
export GAME_DEV_URL="http://127.0.0.1:${GAME_PORT}"
export MODS_DEV_URL="http://127.0.0.1:${MODS_PORT}"

PIDS=()
NAMES=()
SHUTTING_DOWN=0

# Tag every line so three interleaved server logs stay readable.
prefix() {
    local tag="$1"
    while IFS= read -r line; do
        printf '[%s] %s\n' "$tag" "$line"
    done
}

# Process substitution (not a pipeline) so $! is the SERVER's pid, not a
# subshell's - cleanup can then signal the real process instead of orphaning it.
start() {
    local tag="$1"
    shift
    "$@" > >(prefix "$tag") 2>&1 &
    PIDS+=("$!")
    NAMES+=("$tag")
}

cleanup() {
    trap - EXIT INT TERM
    SHUTTING_DOWN=1
    echo ">> stopping ($(IFS=,; echo "${NAMES[*]}"))"
    # By recorded PID only: a pattern kill would reach the other worktrees'
    # preview servers, which is exactly what the random ports are here to avoid.
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    # Trunk and webpack both need a moment to tear down their watchers; escalate
    # only if they overstay.
    local waited=0
    while ((waited < 20)); do
        local alive=0
        for pid in "${PIDS[@]}"; do
            kill -0 "$pid" 2>/dev/null && alive=1
        done
        ((alive)) || break
        sleep 0.25
        ((waited += 1))
    done
    for pid in "${PIDS[@]}"; do
        kill -9 "$pid" 2>/dev/null || true
    done
}
trap cleanup EXIT INT TERM

if [[ ! -d web/node_modules ]]; then
    echo ">> installing web/ dependencies..."
    (cd web && npm install)
fi

serve_site() {
    cd "$ROOT/web"
    exec npm run serve
}

# The developer book: build once so webpack's /dev static mapping has content
# on first load, then keep it fresh with mdbook's own watcher (no port - the
# site dev server serves book/ at /dev/).
echo ">> building the developer book..."
mdbook build
echo ">> starting book watcher (mdbook)..."
start book mdbook watch

echo ">> starting portal (mods) on :${MODS_PORT}..."
start portal "$ROOT/scripts/serve-mods.sh"
echo ">> starting game (trunk) on :${GAME_PORT}..."
start game trunk serve "${TRUNK_ARGS[@]}"
echo ">> starting site (webpack) on :${UI_PORT}..."
start site serve_site

# The first wasm build is slow (minutes from cold), so hold the banner until the
# game answers - otherwise the printed /play/ link 502s on the first click and
# looks broken.
echo ">> waiting for the first build..."
if ! nova_wait_for_port "$GAME_PORT" 900; then
    echo "!! the game did not come up on :${GAME_PORT} - see the [game] log above" >&2
    exit 1
fi
if ! nova_wait_for_port "$UI_PORT" 120; then
    echo "!! the site did not come up on :${UI_PORT} - see the [site] log above" >&2
    exit 1
fi

# Best-effort by design: a box with no browser (headless CI, a bare container)
# must still get a served site, so every failure path here returns 0 and the
# stack keeps running. Detached from stdio and from the job table - an attached
# browser would inherit the terminal and, worse, `wait -n` below would count the
# opener's exit as "a preview server died" and tear the stack down.
open_site() {
    local url="$1"
    local opener
    if command -v xdg-open >/dev/null 2>&1; then
        opener=xdg-open
    elif [[ -n "${BROWSER:-}" ]] && command -v "$BROWSER" >/dev/null 2>&1; then
        opener="$BROWSER"
    else
        echo ">> no xdg-open and no usable \$BROWSER - open ${url} yourself"
        return 0
    fi
    echo ">> opening ${url} (${opener})..."
    "$opener" "$url" </dev/null >/dev/null 2>&1 &
    disown "$!" 2>/dev/null || true
}

cat <<BANNER

>> live preview ready  (Ctrl-C to stop)
     site:   http://localhost:${UI_PORT}/
     game:   http://localhost:${UI_PORT}/play/     (direct: http://localhost:${GAME_PORT}/)
     portal: http://localhost:${UI_PORT}/mods/     (direct: http://localhost:${MODS_PORT}/mods/)
     book:   http://localhost:${UI_PORT}/dev/

BANNER

if ((OPEN_BROWSER)); then
    open_site "http://localhost:${UI_PORT}/"
fi

# Any server dying takes the stack down, rather than leaving a half-wired
# preview that serves stale output from the survivors.
wait -n
if ((SHUTTING_DOWN == 0)); then
    echo "!! a preview server exited - shutting the rest down" >&2
fi
