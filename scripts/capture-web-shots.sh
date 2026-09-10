#!/usr/bin/env bash
# Capture every wiki/tutorial STILL the site ships, then package them.
#
# Usage: scripts/capture-web-shots.sh [example ...]
#
# With no arguments it runs every producer the screenshot manifest names
# (`gen-web-screenshots.py --producers`); with arguments it runs exactly those,
# which is the loop to use while iterating on one framing. Either way it ends
# by running the packager, so `web/src/assets` is up to date when it returns.
#
# Output: <stage>/<name>.png per shot, then the manifest's copies and composites
# under web/src/assets. Stage is target/shots.
#
# Requires: cargo, python3, xvfb-run (all in the flake devshell). The capture
# needs a software Vulkan (lavapipe/llvmpipe) behind the Xvfb display.
#
# The loops have their own driver, `scripts/capture-web-media.sh`; the two share
# this sandbox and its base-only rule.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STAGE="${CARGO_TARGET_DIR:-target}/shots"
mkdir -p "$STAGE"
STAGE="$(cd "$STAGE" && pwd)"

# The site shows MAINLINE, from a clean install. Two of the developer's own
# directories would otherwise leak into the frame: the data directory, whose
# installed mods merge live at load, and the config directory, which carries the
# saved enabled-mod set and the graphics settings.
SANDBOX="${CARGO_TARGET_DIR:-target}/capture-home"
mkdir -p "$SANDBOX/data/mods" "$SANDBOX/config"
SANDBOX="$(cd "$SANDBOX" && pwd)"
export NOVA_MODDING_CACHE_ROOT="$SANDBOX/data"
export NOVA_CONFIG_ROOT="$SANDBOX/config"

# A producer that changes the enabled-mod set SAVES it, and the next producer to
# run reads it back: `screenshot_scenario_picker` enables the example mod on
# purpose, and every shot taken after it then wore that mod's hull override. So
# base-only is re-stamped before each producer rather than seeded once.
base_only_mods() {
    printf '["base"]' >"$SANDBOX/config/enabled_mods.ron"
}

for tool in cargo python3 xvfb-run; do
    command -v "$tool" >/dev/null || {
        echo "!! $tool not on PATH (run inside \`nix develop\`)" >&2
        exit 1
    }
done

if [[ "$#" -gt 0 ]]; then
    producers=("$@")
else
    mapfile -t producers < <(python3 scripts/gen-web-screenshots.py --producers)
    # Process substitution cannot fail the pipeline, and `set -u` is happy with
    # an empty array: without this a packager that refused to name its
    # producers would capture nothing, package the shipped bytes back over
    # themselves and report success.
    [[ "${#producers[@]}" -gt 0 ]] || {
        echo "!! the screenshot manifest named no producers - see the error above" >&2
        exit 1
    }
fi

for example in "${producers[@]}"; do
    echo ">> ${example}: capturing under Xvfb..."
    base_only_mods
    NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR="$STAGE" \
        xvfb-run -a -s "-screen 0 1920x1080x24" \
        cargo run --features debug --example "$example"
done

echo ">> packaging $STAGE -> web/src/assets"
python3 scripts/gen-web-screenshots.py --stage-dir "$STAGE"
