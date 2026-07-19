#!/usr/bin/env bash
# The PROFILED pass of the run-harness (task 20260719-112253): run an
# autopilot example with bevy's per-system tracing compiled in, collect the
# chrome-trace JSON, and render the top-N costliest-systems table. Optionally
# (SAMPLY=1) a SECOND run under samply captures a native flamegraph profile.
#
# This is deliberately a separate pass from the FPS capture
# (scripts/perf-baseline.sh): tracing serialization inflates frame times, so
# a traced run's numbers RANK systems but never stand in for the clean
# pass's frame stats (spike 20260719-112011, review M2).
#
# Usage:
#   scripts/perf-profile.sh [example] [out_dir]
#     example  the autopilot example to drive (default 08_scenario)
#     out_dir  artifact dir (default perf-profile/<example>)
#
# Env:
#   TOP      table size (default 20)
#   SAMPLY   set to 1 to add a samply flamegraph run (skipped with a note
#            when samply is not installed - the pass never fails on a
#            missing profiler)
#   DISPLAY_OVERRIDE  run against this X display instead of a throwaway Xvfb
#
# Artifacts in out_dir: trace.json (open in https://ui.perfetto.dev),
# top-systems.md, and (SAMPLY=1) samply-profile.json.gz (load with
# `samply load`, opens the Firefox Profiler UI).
set -euo pipefail

EXAMPLE="${1:-08_scenario}"
OUT_DIR="${2:-$(pwd)/perf-profile/${EXAMPLE}}"
TOP="${TOP:-20}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
export BEVY_ASSET_ROOT="$REPO_ROOT"
mkdir -p "$OUT_DIR"

echo "== building $EXAMPLE with per-system tracing (--features debug,trace) =="
cargo build --example "$EXAMPLE" --features debug,trace
BIN="target/debug/examples/${EXAMPLE}"

XVFB_PID=""
cleanup() { [ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null || true; }
trap cleanup EXIT
if [ -n "${DISPLAY_OVERRIDE:-}" ]; then
  export DISPLAY="$DISPLAY_OVERRIDE"
else
  echo "== starting Xvfb on :94 =="
  Xvfb :94 -screen 0 1280x720x24 >/dev/null 2>&1 &
  XVFB_PID=$!
  sleep 2
  export DISPLAY=":94"
fi

echo "== traced run: $EXAMPLE -> $OUT_DIR/trace.json =="
# The game's log filter sets bevy_ecs=warn (nova_core log_filter_str) to
# silence ECS log chatter - but the same EnvFilter governs SPANS, so it
# silently kills every per-system span. RUST_LOG directives are ADDED on top
# of the plugin filter (bevy_log 0.19) and a same-target directive wins, so
# this one line is what makes the whole profiled pass produce data.
TRACE_CHROME="$OUT_DIR/trace.json" BCS_AUTOPILOT=1 \
  RUST_LOG="${RUST_LOG:+$RUST_LOG,}bevy_ecs=info" "$BIN"

echo "== top $TOP systems =="
cargo run --quiet -p nova_probe --bin perf_trace -- \
  "$OUT_DIR/trace.json" --top "$TOP" -o "$OUT_DIR/top-systems.md"
cat "$OUT_DIR/top-systems.md"

if [ -n "${SAMPLY:-}" ]; then
  if command -v samply >/dev/null 2>&1; then
    echo "== building $EXAMPLE for sampling (--profile profiling, frame pointers) =="
    # A dedicated build for the flamegraph: the `profiling` cargo profile
    # keeps full DWARF in the binary (the dev profile's line-tables-only +
    # unpacked split leaves samply with raw hex addresses), and frame
    # pointers give the unwinder honest stacks. NO trace feature here on
    # purpose - sampling needs no spans, and span overhead would distort
    # the very costs being sampled. Driver-blob frames (NVIDIA, stripped
    # system libs) stay unsymbolicated regardless; that is theirs, not ours.
    RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C force-frame-pointers=yes" \
      cargo build --example "$EXAMPLE" --features debug --profile profiling
    SAMPLY_BIN="target/profiling/examples/${EXAMPLE}"
    echo "== samply flamegraph run =="
    # Tolerant on purpose: sampling needs perf_event_paranoid <= 1 (samply
    # prints the exact sudo command when blocked) AND enough lockable perf
    # ring-buffer memory - on many-core hosts "mmap failed" means
    # perf_event_mlock_kb is too small for one buffer per CPU (fix:
    # echo 16384 | sudo tee /proc/sys/kernel/perf_event_mlock_kb). A
    # missing/blocked profiler must never fail the pass - the trace + table
    # already landed.
    if BCS_AUTOPILOT=1 samply record --save-only \
      -o "$OUT_DIR/samply-profile.json.gz" "$SAMPLY_BIN"; then
      echo "== load with: samply load $OUT_DIR/samply-profile.json.gz =="
    else
      echo "== samply run FAILED (perms? see message above); flamegraph skipped, pass still OK =="
    fi
  else
    echo "== samply not installed; skipping the flamegraph run =="
  fi
fi

echo
echo "== artifacts in $OUT_DIR =="
ls -la "$OUT_DIR"
