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
# With no arguments it captures every lesson that has a producer, which is
# currently every lesson the handbook authors. Name lessons to capture only
# those. A lesson authored WITHOUT a producer keeps the placeholder art
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
#
# ONE PRODUCER MAY OWN SEVERAL LESSONS. A walk that has already opened a screen
# is the cheapest place to photograph the next one, so a row lists every lesson
# its example writes:
#   example|lesson:kind[,lesson:kind...]
# The example catalog in `Cargo.toml` caps a producer at three frames, so a row
# never grows past three.
PRODUCERS=(
    "lesson_start_welcome|start_welcome:still"
    "lesson_start_scene|start_hud:still,start_camera:loop"
    "lesson_start_dock|start_cinematic:loop,start_verbs:loop"
    "lesson_flight_aim|flight_aim:loop"
    "lesson_flight_basics|flight_momentum:loop,flight_stop:loop,flight_rcs:loop"
    "lesson_flight_orders|flight_goto:loop,flight_orbit:loop"
    "lesson_flight_limits|flight_speedcap:loop,flight_cancel:loop"
    "lesson_flight_well|flight_gravity:loop,flight_arrival:loop"
    "lesson_combat_radar|combat_radar:loop"
    "lesson_combat_moves|combat_stance:loop,combat_components:loop,combat_turrets:still"
    "lesson_combat_field|combat_allegiance:still,combat_cover:loop"
    "lesson_combat_rounds|combat_damage_types:loop"
    "lesson_combat_torpedoes|combat_torpedoes:loop"
    "lesson_build_sections|build_sections:still,build_mass:still,build_balance:still"
    "lesson_build_geometry|build_turning:still,build_weapon_mounts:still,build_docking_port:still"
    "lesson_build_generate|build_generate:loop"
    "lesson_build_flight_test|build_flight_test:loop"
    "lesson_novaos|novaos_open:loop,novaos_view:loop"
    "lesson_novaos_contacts|novaos_contacts:still"
    "lesson_novaos_prompt|novaos_terminal:loop,novaos_commands:loop"
    "lesson_novaos_ship|novaos_service:loop,novaos_rebind_section:loop"
    "lesson_menu_advanced|advanced_scenarios:still,advanced_mods:still,advanced_bindings:still"
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

# The lessons one row writes, as `lesson:kind` words.
row_lessons() {
    local row="$1"
    printf '%s' "${row#*|}" | tr ',' ' '
}

# Whether any lesson on this row was asked for.
row_selected() {
    local row="$1" pair
    for pair in $(row_lessons "$row"); do
        selected "${pair%%:*}" && return 0
    done
    return 1
}

examples=()
for row in "${PRODUCERS[@]}"; do
    IFS='|' read -r example _ <<<"$row"
    row_selected "$row" && examples+=(--example "$example")
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
    IFS='|' read -r example _ <<<"$row"
    row_selected "$row" || continue

    pairs="$(row_lessons "$row")"
    for pair in $pairs; do
        rm -f "$STAGE/${pair%%:*}.png"
    done
    echo ">> ${example}: capturing under Xvfb..."
    # A fresh server per run (-a picks a free display). `cargo run` rather than
    # the built binary so the asset root resolves at the repo, the way every
    # capture flow runs.
    NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$STAGE" \
        xvfb-run -a -s "-screen 0 1920x1080x24" \
        cargo run --features debug --example "$example"

    # Every lesson on the row, even one nobody asked for: a producer writes all
    # of its frames in one walk, so re-encoding the others costs a millisecond
    # and leaves the bundle consistent with the run that just happened.
    for pair in $pairs; do
        lesson="${pair%%:*}"
        kind="${pair##*:}"
        file="$STAGE/${lesson}.png"
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
            ;;
        still)
            # Shot at the capture window, which IS what a still ships at - there
            # is no grid to pay for, so nothing is gained by shrinking it.
            [[ "$size" == "${STILL_W}x${STILL_H}" ]] || {
                echo "!! ${lesson}.png is ${size}, and a still ships at" \
                    "${STILL_W}x${STILL_H}" >&2
                exit 1
            }
            ;;
        *)
            echo "!! ${lesson}: unknown kind '${kind}'" >&2
            exit 1
            ;;
        esac

        encode "$file" "$OUT/${lesson}.webp"
        echo ">> ${lesson}.webp: $(dimensions "$OUT/${lesson}.webp"), \
$(stat -c%s "$OUT/${lesson}.webp") bytes (${example}, ${kind})"
        captured=$((captured + 1))
    done
done

echo ">> ${captured} lesson demonstration(s) in ${OUT}"
echo ">> any lesson with no producer keeps its placeholder (scripts/gen-lesson-media.py)"
