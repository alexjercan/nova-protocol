#!/usr/bin/env bash
# Capture the training handbook's demonstrations: build the lesson_* producers,
# run them armed under Xvfb, and package the result into the base bundle at
# assets/base/training/<lesson>.webp.
#
# A lesson's demonstration is REAL FOOTAGE of the game. The producers are
# ordinary capture examples in the one capture idiom
# (`nova_autopilot::loops`): a loop lesson records its own 4x5 sprite sheet and
# tiles it itself, a still lesson shoots a PNG, and this script only stages,
# validates and encodes - the same division of labour as the web loops
# (scripts/capture-web-media.sh) and the web stills
# (scripts/gen-web-screenshots.py).
#
# The staged capture is PNG because a capture should lose nothing; the file
# that SHIPS is WebP. A sheet whose cells are big enough to fill the Lessons
# pane is about 1.6 MB as PNG and about 150 KB as WebP, and the handbook
# carries one per lesson.
#
#     nix develop -c scripts/capture-lesson-media.sh [lesson ...]
#
# With no arguments it captures every lesson that has a producer. Name lessons
# to capture only those. The lessons with no producer keep the placeholder art
# scripts/gen-lesson-media.py draws, which this never touches.
#
# Requires: cargo, ffmpeg, ffprobe, xvfb-run (all in the flake devshell). The
# capture needs a software Vulkan (lavapipe/llvmpipe) behind the Xvfb display.
#
# The run is mod-free and setting-free: NOVA_MODDING_CACHE_ROOT and
# NOVA_CONFIG_ROOT are pointed at empty directories, so neither an installed
# mod nor a saved preference can dress the ships the handbook shows.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="$ROOT/assets/base/training"
STAGE="${CARGO_TARGET_DIR:-target}/lesson-shots"
mkdir -p "$OUT" "$STAGE"
STAGE="$(cd "$STAGE" && pwd)"

# See scripts/capture-web-media.sh: the handbook shows MAINLINE from a clean
# install, so the developer's own data and config directories stay out of shot.
SANDBOX="${CARGO_TARGET_DIR:-target}/capture-home"
mkdir -p "$SANDBOX/data/mods" "$SANDBOX/config"
SANDBOX="$(cd "$SANDBOX" && pwd)"
export NOVA_MODDING_CACHE_ROOT="$SANDBOX/data"
export NOVA_CONFIG_ROOT="$SANDBOX/config"
printf '["base"]' >"$SANDBOX/config/enabled_mods.ron"

for tool in cargo ffmpeg ffprobe xvfb-run; do
    command -v "$tool" >/dev/null || {
        echo "!! $tool not on PATH (run inside \`nix develop\`)" >&2
        exit 1
    }
done

# The lesson producers, and what each one writes.
#
# `loop` is a 4x5 sheet the producer tiles itself; `still` is a single shot.
# Both arrive at the size they ship at, so this only checks the size and
# encodes. The lesson id IS the file name, in both cases, because that is the
# path the lesson authors.
#   example|lesson|loop|still
PRODUCERS=(
    "lesson_flight_aim|flight_aim|loop"
    "lesson_combat_radar|combat_radar|loop"
    "lesson_build_sections|build_sections|still"
)

# What the authored lessons cut a sheet on
# (crates/nova_authoring/src/base_content/lessons.rs): 4x5 cells of 960x540.
# The game cuts by the AUTHORED grid, so a sheet of another size ships as
# frames cut in the wrong places rather than as a visible error.
SHEET_W=3840
SHEET_H=2700
# What a still lesson ships at: the capture window, kept whole.
STILL_W=1920
STILL_H=1080
# The WebP quality the handbook ships at. High enough that the starfield and
# the HUD type survive the encode, low enough that a sheet stays under the PNG
# it replaced by an order of magnitude.
QUALITY=88

wanted=("$@")
selected() {
    [[ "${#wanted[@]}" -eq 0 ]] && return 0
    local lesson="$1" name
    for name in "${wanted[@]}"; do
        [[ "$name" == "$lesson" ]] && return 0
    done
    return 1
}

examples=()
for row in "${PRODUCERS[@]}"; do
    IFS='|' read -r example lesson _ <<<"$row"
    selected "$lesson" && examples+=(--example "$example")
done
[[ "${#examples[@]}" -gt 0 ]] || {
    echo "!! no producer for: $*" >&2
    exit 1
}

echo ">> building the lesson producers..."
cargo build --features debug "${examples[@]}"

dimensions() {
    ffprobe -v error -select_streams v:0 -show_entries stream=width,height \
        -of csv=p=0:s=x "$1"
}

# The staged PNG, as the WebP the handbook ships. `-preset picture` is libwebp's
# tuning for photographic content, which is what a capture of the game is.
encode() {
    ffmpeg -v error -y -i "$1" -c:v libwebp -quality "$QUALITY" \
        -preset picture -pix_fmt yuv420p "$2"
}

captured=0
for row in "${PRODUCERS[@]}"; do
    IFS='|' read -r example lesson kind <<<"$row"
    selected "$lesson" || continue

    file="$STAGE/${lesson}.png"
    rm -f "$file"
    echo ">> ${example}: capturing ${lesson} under Xvfb..."
    # A fresh server per run (-a picks a free display). `cargo run` rather than
    # the built binary so the asset root resolves at the repo, the way every
    # capture flow runs.
    NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$STAGE" \
        xvfb-run -a -s "-screen 0 1920x1080x24" \
        cargo run --features debug --example "$example"

    [[ -s "$file" ]] || {
        echo "!! ${example} exited cleanly but ${lesson}.png is not in ${STAGE}" >&2
        exit 1
    }

    size="$(dimensions "$file")"
    case "$kind" in
    loop)
        # The producer tiled it, so the size IS the contract with the lesson.
        [[ "$size" == "${SHEET_W}x${SHEET_H}" ]] || {
            echo "!! ${lesson}.png is ${size}, and the lesson cuts it as" \
                "${SHEET_W}x${SHEET_H} - the cells would be cut in the wrong" \
                "places" >&2
            exit 1
        }
        encode "$file" "$OUT/${lesson}.webp"
        ;;
    still)
        # Shot at the capture window, which IS what a still ships at - there is
        # no grid to pay for, so nothing is gained by shrinking it.
        [[ "$size" == "${STILL_W}x${STILL_H}" ]] || {
            echo "!! ${lesson}.png is ${size}, and a still ships at" \
                "${STILL_W}x${STILL_H}" >&2
            exit 1
        }
        encode "$file" "$OUT/${lesson}.webp"
        ;;
    *)
        echo "!! ${lesson}: unknown kind '${kind}'" >&2
        exit 1
        ;;
    esac

    echo ">> ${lesson}.webp: $(dimensions "$OUT/${lesson}.webp"), \
$(stat -c%s "$OUT/${lesson}.webp") bytes (${example}, ${kind})"
    captured=$((captured + 1))
done

echo ">> ${captured} lesson demonstration(s) in ${OUT}"
echo ">> the rest keep their placeholder art (scripts/gen-lesson-media.py)"
