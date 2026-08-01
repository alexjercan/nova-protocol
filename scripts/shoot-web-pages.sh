#!/usr/bin/env bash
# Screenshot every page KIND of the landing site, at desktop and mobile widths.
#
# The site's look is only verifiable by seeing it (lesson `render-output-eyeball`
# - a theme/readability change is unverified until the pages are rendered), and
# no capture rig existed, so this is it. It builds `web/`, serves `web/dist` over
# a plain static server and drives headless chromium over the six page kinds.
#
#     nix develop -c scripts/shoot-web-pages.sh target/web-shots
#
# Output: <outdir>/<kind>-<width>.png, plus a manifest.txt naming the commit and
# the URL behind every file. Capture the same set on two commits and compare the
# pairs at identical crop and scale (lesson `compare-crops-at-one-zoom`).
#
# Requires: node/npm (flake devshell), python3, chromium on PATH.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="${1:-target/web-shots}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

for tool in node npm python3 chromium; do
    command -v "$tool" >/dev/null || {
        echo "!! $tool not on PATH (run inside \`nix develop\`)" >&2
        exit 1
    }
done

# The six page kinds. Every distinct piece of furniture on the site appears in at
# least one: nav+hero+cards (landing), post cards (news index), long prose +
# TOC (a news post), controls tables + keycaps (tutorial), the card grid (wiki
# index), and code + tables + mermaid + sidebar (a wiki dev page).
#   name|path
PAGES=(
    "landing|/"
    "news-index|/news/"
    "news-post|/news/0.9.0/"
    "tutorial|/tutorial/"
    "wiki-index|/wiki/"
    "wiki-dev-page|/wiki/dev/architecture/"
)

# Tall viewports so one shot carries most of a page; mermaid + webfonts settle
# inside the virtual-time budget.
#   label|WIDTHxHEIGHT
VIEWPORTS=(
    "desktop|1440x2400"
    "mobile|390x2200"
)

echo ">> building the landing site (webpack)..."
pushd web >/dev/null
[[ -d node_modules ]] || npm install
npm run build
popd >/dev/null

# A free ephemeral port, so parallel runs never collide.
PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"

SERVER_PID=""
cleanup() {
    # Kill by the RECORDED pid only - never a pattern match.
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo ">> serving web/dist on 127.0.0.1:${PORT}..."
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory web/dist >/dev/null 2>&1 &
SERVER_PID=$!

# Wait for the port to answer rather than sleeping a guessed interval. A server
# that never listens must FAIL here - falling through would hand chromium a
# connection error per page, which it renders (and screenshots) without complaint.
listening=0
for _ in $(seq 1 50); do
    if python3 - "$PORT" <<'PY' 2>/dev/null; then listening=1; break; fi
import socket, sys
s = socket.socket()
s.settimeout(0.2)
sys.exit(0 if s.connect_ex(("127.0.0.1", int(sys.argv[1]))) == 0 else 1)
PY
    sleep 0.2
done
[[ "$listening" -eq 1 ]] || {
    echo "!! static server never started listening on ${PORT}" >&2
    exit 1
}
kill -0 "$SERVER_PID" 2>/dev/null || {
    echo "!! static server died" >&2
    exit 1
}

PROFILE="$(mktemp -d)"
trap 'cleanup; rm -rf "$PROFILE"' EXIT

MANIFEST="$OUT/manifest.txt"
{
    echo "# shot at commit $(git rev-parse --short HEAD) on $(git rev-parse --abbrev-ref HEAD)"
    echo "# file<TAB>viewport<TAB>url"
} >"$MANIFEST"

# chromium exits 0 and writes a perfectly good PNG of an ERROR PAGE when a URL
# 404s, so the capture itself can never tell us the page exists. Ask the server
# first: a stale entry in PAGES must fail the run loudly, not quietly produce a
# full set of screenshots of "404: File not found".
assert_200() {
    local status
    status="$(python3 - "$1" <<'PY'
import sys, urllib.error, urllib.request
try:
    with urllib.request.urlopen(sys.argv[1], timeout=10) as r:
        print(r.status)
except urllib.error.HTTPError as e:
    print(e.code)
except Exception as e:  # connection refused, timeout, ...
    print(f"ERR {e}")
PY
)"
    [[ "$status" == "200" ]] || {
        echo "!! $1 returned ${status} - fix the PAGES entry or the build" >&2
        exit 1
    }
}

shots=0
for page in "${PAGES[@]}"; do
    name="${page%%|*}"
    path="${page#*|}"
    url="http://127.0.0.1:${PORT}${path}"
    assert_200 "$url"
    for vp in "${VIEWPORTS[@]}"; do
        label="${vp%%|*}"
        size="${vp#*|}"
        file="$OUT/${name}-${label}.png"
        echo ">> ${name} @ ${label} (${size})"
        # --virtual-time-budget lets the deferred work (webfonts, mermaid's
        # dynamic import, the wiki sidebar render) finish before the capture.
        chromium \
            --headless=new \
            --no-sandbox \
            --disable-gpu \
            --hide-scrollbars \
            --force-device-scale-factor=1 \
            --user-data-dir="$PROFILE" \
            --window-size="${size/x/,}" \
            --virtual-time-budget=8000 \
            --screenshot="$file" \
            "$url" >/dev/null 2>&1
        [[ -s "$file" ]] || {
            echo "!! no capture written for ${name} @ ${label}" >&2
            exit 1
        }
        printf '%s\t%s\t%s\n' "$(basename "$file")" "$label" "$path" >>"$MANIFEST"
        shots=$((shots + 1))
    done
done

expected=$((${#PAGES[@]} * ${#VIEWPORTS[@]}))
[[ "$shots" -eq "$expected" ]] || {
    echo "!! captured ${shots} of ${expected} shots" >&2
    exit 1
}

echo ">> ${shots} captures in ${OUT} (manifest.txt lists them)"
