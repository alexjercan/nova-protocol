#!/usr/bin/env bash
# Frame-time baseline sweep for the v0.7.0 perf task (20260716-123551).
#
# Builds the 20_perf_baseline example once (release), then runs it across the
# heavy scenes x graphics presets, capturing percentile frame-time stats into a
# results directory (one <label>.json per run plus an aggregated frametime.csv).
#
# Renderer:
#   - "gpu" (default): the real discrete GPU rendering into a HEADLESS Xvfb
#     window (Vulkan WSI). No compositor, no vsync gating, no visible window -
#     the clean, repeatable rig this task's report used. (The live desktop :0
#     vsync-clamps and contends, so it is a poor benchmark surface; pass
#     DISPLAY_OVERRIDE=:0 to force it anyway.)
#   - "sw": a software-raster floor via a forced lavapipe (llvmpipe) Vulkan ICD
#     under Xvfb. The worst-case CPU/fill floor that brackets weak hardware; NOT
#     a browser-WebGPU stand-in. Much slower per frame - small counts.
#
# Usage:
#   scripts/perf-baseline.sh [gpu|sw] [out_dir]
#
# Env overrides:
#   SCENARIOS        space-separated scenario ids (default: the three heavy scenes)
#   PRESETS          space-separated graphics presets (default: high low)
#   DISPLAY_OVERRIDE run against this X display instead of a throwaway Xvfb
#   WARMUP           warm-up frames (default: 300; sw defaults lower)
#   FRAMES           captured frames (default: 600; sw defaults lower)
#   RES              forced window resolution WxH (default: 1280x720)
#   LVP_ICD          lavapipe ICD path (sw mode)
set -euo pipefail

RENDERER="${1:-gpu}"
OUT_DIR="${2:-$(pwd)/perf-results/${RENDERER}}"
SCENARIOS="${SCENARIOS:-asteroid_field broadside shakedown_run}"
PRESETS="${PRESETS:-high low}"
RES="${RES:-1280x720}"
LVP_ICD="${LVP_ICD:-/run/opengl-driver/share/vulkan/icd.d/lvp_icd.x86_64.json}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
export BEVY_ASSET_ROOT="$REPO_ROOT"   # bare binary resolves assets/ beside the exe otherwise
mkdir -p "$OUT_DIR"

echo "== building 20_perf_baseline (release) =="
cargo build --release --example 20_perf_baseline --features debug
BIN="target/release/examples/20_perf_baseline"

# Stand up a throwaway Xvfb unless the caller pinned a display.
XVFB_PID=""
cleanup() { [ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null || true; }
trap cleanup EXIT

if [ -n "${DISPLAY_OVERRIDE:-}" ]; then
  export DISPLAY="$DISPLAY_OVERRIDE"
else
  SW_DISPLAY=":95"
  echo "== starting Xvfb on $SW_DISPLAY =="
  Xvfb "$SW_DISPLAY" -screen 0 "${RES}x24" >/dev/null 2>&1 &
  XVFB_PID=$!
  sleep 2
  export DISPLAY="$SW_DISPLAY"
fi

if [ "$RENDERER" = "sw" ]; then
  WARMUP="${WARMUP:-20}"
  FRAMES="${FRAMES:-120}"
  export VK_ICD_FILENAMES="$LVP_ICD" VK_DRIVER_FILES="$LVP_ICD" WGPU_BACKEND=vulkan
else
  WARMUP="${WARMUP:-300}"
  FRAMES="${FRAMES:-600}"
fi

echo "== sweep: renderer=$RENDERER display=$DISPLAY res=$RES warmup=$WARMUP frames=$FRAMES =="
for scenario in $SCENARIOS; do
  for preset in $PRESETS; do
    label="${scenario}-${preset}"
    echo "-- $label --"
    NOVA_PERF=1 \
    NOVA_PERF_SCENARIO="$scenario" \
    NOVA_PERF_QUALITY="$preset" \
    NOVA_PERF_LABEL="$label" \
    NOVA_PERF_OUT="$OUT_DIR" \
    NOVA_PERF_WARMUP="$WARMUP" \
    NOVA_PERF_FRAMES="$FRAMES" \
    NOVA_PERF_RES="$RES" \
      "$BIN" 2>&1 | grep -E "nova perf: label=|panic|ERROR" || true
  done
done

echo
echo "== results in $OUT_DIR =="
[ -f "$OUT_DIR/frametime.csv" ] && column -s, -t "$OUT_DIR/frametime.csv"
