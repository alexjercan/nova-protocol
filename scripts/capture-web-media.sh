#!/usr/bin/env bash
# Capture the docs site's webm loops: build the loop_* capture examples, run
# them armed under Xvfb, and package the encoded webms + a manifest into the
# site's asset tree.
#
# The loops are authored as capture examples in the one capture idiom
# (`nova_autopilot::loops`): each example records its own frames between its
# `loop_start`/`loop_end` steps and encodes `<loop>.webm` into NOVA_CAPTURE_DIR
# itself, so this script only stages, runs, validates and copies - the same
# division of labour as the stills (examples shoot, gen-web-screenshots.py
# packages).
#
#     nix develop -c scripts/capture-web-media.sh [outdir]
#
# Output: <outdir>/<loop>.webm plus a manifest.txt naming the commit and, per
# file, the producing example, duration and size. Default outdir is the
# shipped asset location, web/src/assets/loops.
#
# Requires: cargo, ffprobe, xvfb-run (all in the flake devshell). The capture
# needs a software Vulkan (lavapipe/llvmpipe) behind the Xvfb display.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="${1:-web/src/assets/loops}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

STAGE="${CARGO_TARGET_DIR:-target}/loop-shots"
mkdir -p "$STAGE"
STAGE="$(cd "$STAGE" && pwd)"

for tool in cargo ffprobe xvfb-run; do
    command -v "$tool" >/dev/null || {
        echo "!! $tool not on PATH (run inside \`nix develop\`)" >&2
        exit 1
    }
done

# The loop producers and the loop each writes. One example can in principle
# stage several loops; list one line per loop so the manifest and the size
# gate see each file.
#   example|loop
LOOPS=(
    "loop_torpedo_blast|torpedo-blast"
    "loop_spine_cut|spine-cut"
)

# Per-file budget, bytes. The encode targets 2-3 MB (LOOP_CRF in
# nova_autopilot::loops); a loop over budget FAILS the run - re-cut it or
# raise the CRF, do not ship it heavy.
MAX_BYTES=$((3 * 1024 * 1024))

examples=()
for pair in "${LOOPS[@]}"; do
    examples+=(--example "${pair%%|*}")
done

echo ">> building the loop producers..."
cargo build --features debug "${examples[@]}"

ran=()
for pair in "${LOOPS[@]}"; do
    example="${pair%%|*}"
    loop="${pair#*|}"
    file="$STAGE/${loop}.webm"
    rm -f "$file"

    # Each example runs at most once even when it stages several loops.
    already=0
    for done_example in "${ran[@]:-}"; do
        [[ "$done_example" == "$example" ]] && already=1
    done
    if [[ "$already" -eq 0 ]]; then
        echo ">> ${example}: capturing under Xvfb..."
        # A fresh server per run (-a picks a free display). `cargo run` rather
        # than the built binary so the asset root resolves at the repo, the
        # way every capture flow runs (the build above makes this a pure run).
        # The armed run is frame-clocked, records its frames, encodes its
        # webm(s) and exits through the completion protocol - a nonzero exit
        # here IS a failed capture (open loop, frame cap, ffmpeg failure,
        # stalled step).
        NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$STAGE" \
            xvfb-run -a -s "-screen 0 1920x1080x24" \
            cargo run --features debug --example "$example"
        ran+=("$example")
    fi

    [[ -s "$file" ]] || {
        echo "!! ${example} exited cleanly but ${loop}.webm is not in ${STAGE}" >&2
        exit 1
    }
done

MANIFEST="$OUT/manifest.txt"
{
    echo "# captured at commit $(git rev-parse --short HEAD) on $(git rev-parse --abbrev-ref HEAD)"
    echo "# file<TAB>example<TAB>duration_s<TAB>bytes"
} >"$MANIFEST"

for pair in "${LOOPS[@]}"; do
    example="${pair%%|*}"
    loop="${pair#*|}"
    file="$STAGE/${loop}.webm"

    duration="$(ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 "$file")"
    width="$(ffprobe -v error -select_streams v:0 -show_entries stream=width -of default=nw=1:nk=1 "$file")"
    height="$(ffprobe -v error -select_streams v:0 -show_entries stream=height -of default=nw=1:nk=1 "$file")"
    bytes="$(stat -c%s "$file")"

    [[ "$width" == "1280" && "$height" == "720" ]] || {
        echo "!! ${loop}.webm is ${width}x${height}, expected 1280x720" >&2
        exit 1
    }
    [[ "$bytes" -le "$MAX_BYTES" ]] || {
        echo "!! ${loop}.webm is ${bytes} bytes (budget ${MAX_BYTES}) - re-cut the loop or raise LOOP_CRF" >&2
        exit 1
    }

    cp "$file" "$OUT/${loop}.webm"
    printf '%s\t%s\t%.1f\t%s\n' "${loop}.webm" "$example" "$duration" "$bytes" >>"$MANIFEST"
    echo ">> ${loop}.webm: ${duration%.*}s, ${bytes} bytes (${example})"
done

echo ">> $((${#LOOPS[@]})) loop(s) in ${OUT} (manifest.txt lists them)"
