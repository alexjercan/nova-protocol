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
#
# The run is mod-free and setting-free: NOVA_MODDING_CACHE_ROOT and
# NOVA_CONFIG_ROOT are pointed at empty directories, so neither an installed
# mod nor a saved preference can dress the ships the site shows.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="${1:-web/src/assets/loops}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

STAGE="${CARGO_TARGET_DIR:-target}/loop-shots"
mkdir -p "$STAGE"
STAGE="$(cd "$STAGE" && pwd)"

# The site shows MAINLINE, from a clean install. Two of the developer's own
# directories would otherwise leak into the frame: the data directory, whose
# installed mods merge live at load (a box with The Ledger installed reskins
# the fleet these loops are of), and the config directory, which carries the
# saved enabled-mod set and the graphics settings. Both are pointed at empty
# directories of our own, so the run sees base and the shipped defaults
# whatever the host has.
SANDBOX="${CARGO_TARGET_DIR:-target}/capture-home"
mkdir -p "$SANDBOX/data/mods" "$SANDBOX/config"
SANDBOX="$(cd "$SANDBOX" && pwd)"
export NOVA_MODDING_CACHE_ROOT="$SANDBOX/data"
export NOVA_CONFIG_ROOT="$SANDBOX/config"

# The sandbox is not write-only: a producer that changes the enabled-mod set
# SAVES it, and the next producer to run reads it back. `screenshot_scenario_picker`
# enables the example mod on purpose - its whole subject is a campaign folded
# under a header - and every shot taken after it then wore the example mod's
# hull override. So base-only is re-stamped before each producer rather than
# seeded once.
base_only_mods() {
    printf '["base"]' >"$SANDBOX/config/enabled_mods.ron"
}

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
# The arena's two loops name a STYLE per side: AMBER flies armoured plate,
# ONYX salvage. Both are shipped base styles, and a named look is what makes
# the two sides read apart at knife range, where the team chevrons are the only
# other thing telling them apart.
LOOPS=(
    "wfc_arena|hero-wfc-duel|--ship amber:armoured --ship onyx:salvage|"
    "wfc_arena|landing-wfc-2v2|--ship amber:armoured --ship amber:armoured --ship onyx:salvage --ship onyx:salvage|"
    "loop_torpedo_blast|torpedo-blast||"
    "loop_spine_cut|spine-cut||"
    "loop_goto_arrival|goto-arrival||"
    "loop_player_flight|landing-player-flight||"
    "loop_derived_skin|news-0110-derived-skin||"
    "loop_round_types|news-0110-round-types||"
    "system_torpedo_launch|loop-section-torpedo-bay||"
    "system_torpedo_launch|news-0110-torpedo-types||"
    "stress_point_defense|loop-section-turret||NOVA_STRESS_PD_MOUNTS=4 NOVA_STRESS_PD_BAYS=4 NOVA_STRESS_PD_VIEW=lanes"
    "loop_turret_stow|loop-section-turret-stow||"
    "loop_cockpit|landing-cockpit||"
    "loop_command_shell|command-shell-open||"
    "screenshot_flip_burn|loop-section-controller||"
    "screenshot_radar_lock|lock-dwell||"
    "screenshot_editor|landing-editor-build||"
    "screenshot_editor|news-0110-editor-skin||"
    "screenshot_damage_levels|loop-section-hull||"
    "screenshot_editor|news-0120-editor-events||"
    "loop_vfx_range|vfx-range||"
    "loop_vfx_range|vfx-cold-launch||"
    "loop_damage_sequence|landing-damage-sequence||"
    "screenshot_railgun|loop-section-railgun||NOVA_RAILGUN_AFTERMATH=0.5"
    "screenshot_railgun|loop-section-railgun-live||NOVA_RAILGUN_LIVE=1 NOVA_RAILGUN_AFTERMATH=2.0"
    "railgun_wake_bench|news-0130-railgun-wake||NOVA_WAKE_LOOP=1"
    "stress_hull_collapse|news-0130-hull-collapse||NOVA_COLLAPSE_LOOP=1"
    "system_lock_line_of_sight|news-0130-lock-occlusion||NOVA_SIGHT_LOOP=1"
    "loop_hull_generate|news-0130-hull-generate||"
    "loop_helm_orders|news-0130-helm-orders||"
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
    "news-0120-release-lead|landing-editor-build|screenshot_editor"
    "news-0120-point-defense|loop-section-turret|stress_point_defense"
    "news-0120-blast|torpedo-blast|loop_torpedo_blast"
    "news-0120-cold-launch|vfx-cold-launch|loop_vfx_range"
    "news-0120-vfx-range|vfx-range|loop_vfx_range"
    # The v0.13.0 post. Interim: the lead and the duel both read the arena
    # hero until the post gets a lead cut of its own; the cladding figure
    # reads the landing damage sequence, which sheds plates since 615b239c.
    "news-0130-release-lead|hero-wfc-duel|wfc_arena"
    "news-0130-block-duel|hero-wfc-duel|wfc_arena"
    "news-0130-railgun-commit|loop-section-railgun|screenshot_railgun"
    "news-0130-railgun-corridor|loop-section-railgun-live|screenshot_railgun"
    "news-0130-pdc-stow|loop-section-turret-stow|loop_turret_stow"
    "news-0130-bay-iris|loop-section-torpedo-bay|system_torpedo_launch"
    "news-0130-cladding|landing-damage-sequence|loop_damage_sequence"
    "news-0130-command-shell|command-shell-open|loop_command_shell"
    "news-0130-torpedo-blast|torpedo-blast|loop_torpedo_blast"
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
# packaged into nothing; it only reports. Empty: every loop a page asks for has
# a producer.
#   loop|why
PENDING=(
    "news-0130-goto-standoff|loop_goto_standoff records it, but GOTO's arrival creep parks the gunship inside the orb at any margin the frame can show (its park log has the numbers)"
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
    base_only_mods
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

# Loops cut OUTSIDE a producer and checked in as they are: a window of a bench
# movie (`bench play --record`), a split frame cut in content-machine. Nothing
# stages them, so a sweep only reads them back out of the shipped tree; they
# are news loops, so they are frozen by definition. The middle field names
# where the footage came from and stands in the manifest's example column.
#   loop|source|why
IMPORTED=(
    "news-0130-agent-run|bench-replay|cut from the recorded tutorial play in bench-runs/395a8f1b, wire lines overlaid from its audit"
    "news-0130-cinematic|bench-replay|the opening of the same play: the title card, the trainer held in frame, the handback"
    "news-0130-rcs-box|bench-replay|the same play's RCS lesson: the slide, then STOP"
    "news-0130-basic-training|bench-replay|the same play's GOTO leg: the card, the chip, the park, the card that follows"
    "news-0130-sections-before-after|loop_sections_compare|the left half is the content-machine v0.12.0 capsule scene sections-compare, composed by hand"
    "news-0130-belt-before-after|loop_belt_compare|the left half is the content-machine v0.12.0 capsule scene belt-compare, composed by hand"
    "news-0130-death-before-after|loop_death_compare|the left half is the content-machine v0.12.0 capsule scene death-compare, composed by hand"
)

package_import() {
    local loop="$1" source="$2" file duration bytes
    file="$OUT/${loop}.webm"
    [[ -s "$file" ]] || {
        echo "!! imported ${loop}.webm is not in ${OUT}: it is cut by hand, so nothing can restage it" >&2
        exit 1
    }
    duration="$(ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 "$file")"
    bytes="$(stat -c%s "$file")"
    [[ "$bytes" -le "$MAX_BYTES" ]] || {
        echo "!! ${loop}.webm is ${bytes} bytes (budget ${MAX_BYTES}) - re-cut the import" >&2
        exit 1
    }
    printf '%s\t%s\t%.1f\t%s\t%s\n' \
        "${loop}.webm" "$source" "$duration" "$bytes" "frozen" >>"$MANIFEST"
    echo ">> ${loop}.webm: ${duration%.*}s, ${bytes} bytes (${source}, imported)"
}

for import in "${IMPORTED[@]}"; do
    IFS='|' read -r loop source _ <<<"$import"
    package_import "$loop" "$source"
done

count=$((${#LOOPS[@]} + ${#ALIASES[@]} + ${#IMPORTED[@]}))
echo ">> ${count} loop(s) in ${OUT} (manifest.txt lists them)"
[[ "${#PENDING[@]}" -eq 0 ]] || echo ">> ${#PENDING[@]} loop(s) still pending a producer"
