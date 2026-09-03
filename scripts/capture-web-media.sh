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
# needs a software Vulkan (lavapipe/llvmpipe) behind the Xvfb display. Set
# NOVA_REUSE_STAGE=1 to repackage a completed target/loop-shots capture set.
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

# The loop producers and the loop each writes. The optional third field is the
# producer's argument string; the optional fourth is a space-separated list of
# environment assignments for that composition.
#   example|loop|args|environment
LOOPS=(
    "wfc_arena|hero-wfc-duel||"
    "wfc_arena|landing-wfc-2v2|--ship amber --ship amber --ship onyx --ship onyx|"
    "loop_torpedo_blast|torpedo-blast||"
    "loop_spine_cut|spine-cut||"
    "loop_goto_arrival|goto-arrival||"
    "loop_player_flight|landing-player-flight||"
    "loop_derived_skin|news-0110-derived-skin||"
    "loop_round_types|news-0110-round-types||"
    "system_torpedo_launch|loop-section-torpedo-bay||"
    "stress_point_defense|loop-section-turret||NOVA_STRESS_PD_MOUNTS=4 NOVA_STRESS_PD_BAYS=4 NOVA_STRESS_PD_VIEW=lanes"
    "loop_cockpit|landing-cockpit||"
    "screenshot_flip_burn|loop-section-controller||"
    "screenshot_radar_lock|lock-dwell||"
    "screenshot_editor|landing-editor-build||"
    "screenshot_editor|news-0110-editor-skin||"
    "screenshot_damage_levels|loop-section-hull||"
    "screenshot_editor|news-0120-editor-events||"
    "loop_vfx_range|vfx-range||"
    "loop_vfx_range|vfx-cold-launch||"
    "loop_damage_sequence|landing-damage-sequence||"
)

# A second name for footage already captured above, when a second producer
# would only record the same thing again.
#
# A `news-` name is always a LEAF: it may be an alias destination, never a
# source, and no page outside its own post may show it (web/tests/
# assets.test.js holds that end). The reason is that a news loop is frozen -
# it records what one release looked like and is never re-cut - so anything
# sourcing from it inherits a freeze it did not ask for. Three wiki section
# loops used to, and would have broken silently the day their post's entry
# was retired.
#   destination|source|producer
ALIASES=(
    "nova-os-open|landing-cockpit|loop_cockpit"
    "loop-section-thruster|landing-player-flight|loop_player_flight"
    "news-0110-release-lead|spine-cut|loop_spine_cut"
    "news-0110-spine-cut|spine-cut|loop_spine_cut"
    "news-0110-parts-gallery|landing-editor-build|screenshot_editor"
    "news-0110-damage-levels|loop-section-hull|screenshot_damage_levels"
    "news-0110-point-defense|loop-section-turret|stress_point_defense"
    "news-0110-torpedo-types|loop-section-torpedo-bay|system_torpedo_launch"
    "news-0120-release-lead|landing-editor-build|screenshot_editor"
    "news-0120-point-defense|loop-section-turret|stress_point_defense"
    "news-0120-blast|torpedo-blast|loop_torpedo_blast"
    "news-0120-cold-launch|vfx-cold-launch|loop_vfx_range"
    "news-0120-vfx-range|vfx-range|loop_vfx_range"
)

for alias in "${ALIASES[@]}"; do
    IFS='|' read -r destination source _ <<<"$alias"
    [[ "$source" != news-* ]] || {
        echo "!! alias ${destination} sources ${source}: a news loop is frozen," \
            "so nothing may derive from it - point it at the living loop" >&2
        exit 1
    }
done

# Loops a page asks for that NO producer can record yet, and why. Listed rather
# than left out, so a full pass NAMES the gap instead of leaving a page's
# placeholder unexplained - the same rule the pending stills follow in
# scripts/gen-web-screenshots.py. A pending row is captured by nothing and
# packaged into nothing; it only reports.
#   loop|why
PENDING=(
    "loop-section-railgun|no capture example flies a hull carrying a lance, so nothing can record the bore, the shot and the corridor in one pass"
)

# Per-file budget, bytes. The encode targets 2-3 MB (LOOP_CRF in
# nova_autopilot::loops); a loop over budget FAILS the run - re-cut it or
# raise the CRF, do not ship it heavy.
MAX_BYTES=$((3 * 1024 * 1024))

examples=()
for pair in "${LOOPS[@]}"; do
    IFS='|' read -r example _ _ <<<"$pair"
    examples+=(--example "$example")
done

echo ">> building the loop producers..."
cargo build --features debug "${examples[@]}"

for pair in "${LOOPS[@]}"; do
    IFS='|' read -r example loop arg_string env_string <<<"$pair"
    file="$STAGE/${loop}.webm"
    if [[ "${NOVA_REUSE_STAGE:-0}" == "1" && -s "$file" ]]; then
        echo ">> ${example}: reusing staged ${loop}"
        continue
    fi
    rm -f "$file"
    read -r -a run_args <<<"$arg_string"
    read -r -a run_env <<<"$env_string"

    echo ">> ${example}: capturing ${loop} under Xvfb..."
    # A fresh server per run (-a picks a free display). `cargo run` rather
    # than the built binary so the asset root resolves at the repo, the way
    # every capture flow runs. Each tuple runs independently because one
    # producer can select a different composition through its arguments.
    if [[ "${#run_args[@]}" -gt 0 ]]; then
        env "${run_env[@]}" NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$STAGE" \
            xvfb-run -a -s "-screen 0 1920x1080x24" \
            cargo run --features debug --example "$example" -- "${run_args[@]}"
    else
        env "${run_env[@]}" NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$STAGE" \
            xvfb-run -a -s "-screen 0 1920x1080x24" \
            cargo run --features debug --example "$example"
    fi

    [[ -s "$file" ]] || {
        echo "!! ${example} exited cleanly but ${loop}.webm is not in ${STAGE}" >&2
        exit 1
    }
done

for alias in "${ALIASES[@]}"; do
    IFS='|' read -r destination source _ <<<"$alias"
    cp "$STAGE/${source}.webm" "$STAGE/${destination}.webm"
done

# A shipped news loop is FROZEN, the same rule the stills packager applies
# (`frozen` in scripts/gen-web-screenshots.py). The living loops - the landing
# page, the wiki section pages - show what the game does now and are re-cut
# every cycle. A news loop is the evidence for what one release changed, so
# re-cutting it hands an old post footage of a game it was never about.
#
# The producers still RUN and still stage: an alias for the current cycle reads
# its source out of the stage, so skipping the run would break it. Only the
# copy into the shipped tree is held. NOVA_UNFREEZE takes a comma-separated
# prefix list to re-open the post being authored (`NOVA_UNFREEZE=news-0120`).
is_frozen() {
    local loop="$1" prefix
    [[ "$loop" == news-* ]] || return 1
    [[ -s "$OUT/${loop}.webm" ]] || return 1
    IFS=',' read -r -a unfrozen <<<"${NOVA_UNFREEZE:-}"
    for prefix in "${unfrozen[@]}"; do
        [[ -n "$prefix" && "$loop" == "$prefix"* ]] && return 1
    done
    return 0
}

MANIFEST="$OUT/manifest.txt"
{
    echo "# captured at commit $(git rev-parse --short HEAD) on $(git rev-parse --abbrev-ref HEAD)"
    echo "# a frozen row shipped with an earlier post and predates that commit"
    echo "# file<TAB>example<TAB>duration_s<TAB>bytes<TAB>state"
} >"$MANIFEST"

package_loop() {
    local loop="$1" example="$2" file duration width height bytes state
    if is_frozen "$loop"; then
        file="$OUT/${loop}.webm"
        state=frozen
    else
        file="$STAGE/${loop}.webm"
        state=fresh
    fi
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

    if [[ "$state" == fresh ]]; then
        cp "$file" "$OUT/${loop}.webm"
    fi
    printf '%s\t%s\t%.1f\t%s\t%s\n' \
        "${loop}.webm" "$example" "$duration" "$bytes" "$state" >>"$MANIFEST"
    echo ">> ${loop}.webm: ${duration%.*}s, ${bytes} bytes (${example}, ${state})"
}

for pair in "${LOOPS[@]}"; do
    IFS='|' read -r example loop _ <<<"$pair"
    package_loop "$loop" "$example"
done
for alias in "${ALIASES[@]}"; do
    IFS='|' read -r loop _ example <<<"$alias"
    package_loop "$loop" "$example"
done

for slot in "${PENDING[@]}"; do
    IFS='|' read -r loop why <<<"$slot"
    printf '%s\t%s\t%s\t%s\t%s\n' "${loop}.webm" "-" "0.0" "0" "pending" >>"$MANIFEST"
    echo ">> ${loop}.webm: PENDING - ${why}"
done

count=$((${#LOOPS[@]} + ${#ALIASES[@]}))
echo ">> ${count} loop(s) in ${OUT} (manifest.txt lists them)"
[[ "${#PENDING[@]}" -eq 0 ]] || echo ">> ${#PENDING[@]} loop(s) still pending a producer"
