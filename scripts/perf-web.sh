#!/usr/bin/env bash
# Web / WebGPU frame-time capture for the v0.7.0 perf task (20260716-123551).
#
# Builds the perf_web bin to wasm (Trunk, WebGPU), serves it, drives a headless
# Chromium at the perf URL, and scrapes the `nova perf: label=...` line the wasm
# logs to the browser console (there is no filesystem on web).
#
# Usage:
#   scripts/perf-web.sh [scenario]           # default asteroid_field
# Env:
#   QUALITY  low|medium|high (default high)
#   COMBAT   set to 1 to drive a combat burst (use a combat scenario)
#   FRAMES   captured frames (default 600)
#   WARMUP   warm-up frames (default 180)
#   PORT     static server port (default 8099)
#   HEADLESS set to 1 to try --headless=new instead of Xvfb (headless WebGPU is
#            flaky on Linux; the default runs Chromium under Xvfb+GPU, which is
#            the proven path here - WebGPU needs the GPU process + a real origin)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SCENARIO="${1:-asteroid_field}"
QUALITY="${QUALITY:-high}"
FRAMES="${FRAMES:-600}"
WARMUP="${WARMUP:-180}"
PORT="${PORT:-8099}"
DIST="dist-perf"
LABEL="${SCENARIO}-${QUALITY}${COMBAT:+-combat}-web"

echo ">> trunk build (release) perf.html -> $DIST"
trunk build --release -d "$DIST" perf.html

echo ">> serving $DIST on :$PORT"
python3 -m http.server -d "$DIST" "$PORT" >/dev/null 2>&1 &
SRV=$!
CHROME=""
XVFB_PID=""
cleanup() { kill "$SRV" ${CHROME:+$CHROME} ${XVFB_PID:+$XVFB_PID} 2>/dev/null || true; }
trap cleanup EXIT
sleep 1

# Trunk emits the entry as index.html (even from perf.html), served at /.
URL="http://localhost:${PORT}/?perf=1&scenario=${SCENARIO}&quality=${QUALITY}&frames=${FRAMES}&warmup=${WARMUP}&label=${LABEL}${COMBAT:+&combat=1}"
echo ">> $URL"

LOG="$(mktemp)"
# WebGPU needs the GPU process + Vulkan + a non-blocklisted adapter; these flags
# get a real NVIDIA adapter here (probed: "ADAPTER OK 21 features").
CHROME_FLAGS=(
  --no-sandbox --disable-gpu-sandbox --ignore-gpu-blocklist
  --enable-unsafe-webgpu --enable-features=Vulkan,WebGPU --use-angle=vulkan
  --enable-logging=stderr --v=1 --window-size=1280,720
)
if [ "${HEADLESS:-}" = "1" ]; then
  chromium --headless=new "${CHROME_FLAGS[@]}" "$URL" >"$LOG" 2>&1 &
  CHROME=$!
else
  Xvfb :95 -screen 0 1280x720x24 >/dev/null 2>&1 &
  XVFB_PID=$!
  sleep 2
  DISPLAY=:95 chromium "${CHROME_FLAGS[@]}" "$URL" >"$LOG" 2>&1 &
  CHROME=$!
fi

echo ">> waiting for capture (up to 150s)..."
for _ in $(seq 1 150); do
  grep -q "nova perf: label=${LABEL} frames" "$LOG" && break
  kill -0 "$CHROME" 2>/dev/null || { echo "!! chromium exited early"; break; }
  sleep 1
done

echo "=== result ($LABEL) ==="
grep -aoE "nova perf: label=${LABEL} frames.*|AdapterInfo \{[^}]*\}|Requesting adapter|no suitable|panicked.*|failed to (create|get)" "$LOG" | head -20 \
  || echo "no perf line captured"
cp "$LOG" /tmp/perf-web-last.log
echo ">> full chromium log: /tmp/perf-web-last.log"
