#!/usr/bin/env bash
# usage: launch.sh <run-dir> <capture:1|0>
set -u
D="$1"; CAP="$2"
mkdir -p "$D/xdg/config" "$D/xdg/data" "$D/xdg/cache" "$D/xdg/state"
cd /home/alex/personal/nova-protocol
ENVV=(
  -u DISPLAY -u WAYLAND_DISPLAY -u NOVA_CAPTURE -u NOVA_CAPTURE_DIR
  NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=2400
  NOVA_PROBE_TIMELINE="$D/timeline.jsonl"
  VK_DRIVER_FILES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
  ALSA_CONFIG_PATH=/dev/null
  XDG_CONFIG_HOME="$D/xdg/config" XDG_DATA_HOME="$D/xdg/data"
  XDG_CACHE_HOME="$D/xdg/cache" XDG_STATE_HOME="$D/xdg/state"
  RUST_LOG=info
)
if [ "$CAP" = 1 ]; then ENVV+=(NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$D/captures"); fi
CMD=(env "${ENVV[@]}" nix develop --command xvfb-run -a -s "-screen 0 1920x1080x24" cargo run --example system_job_ammo_economics --features debug)
printf '%q ' "${CMD[@]}" > "$D/command.txt"; echo >> "$D/command.txt"
git rev-parse HEAD > "$D/head.txt"; git status --porcelain > "$D/status-before.txt"
echo "START $(date -u +%FT%T.%NZ)" > "$D/times.txt"
"${CMD[@]}" > "$D/run.log" 2>&1
echo "EXIT=$?" > "$D/exit.txt"
echo "END $(date -u +%FT%T.%NZ)" >> "$D/times.txt"
git status --porcelain > "$D/status-after.txt"
