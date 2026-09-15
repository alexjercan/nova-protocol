#!/usr/bin/env bash
# Launch the worktree's mod without touching the player's installed copy.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
case "${1:-1}" in
    1) scenario=ledger_01_drift_run ;;
    2) scenario=ledger_02_rock_garden ;;
    3) scenario=ledger_03_blue_survey ;;
    4) scenario=ledger_04_cold_patrol ;;
    5) scenario=ledger_05_freight_lane ;;
    6) scenario=ledger_06_siege_line ;;
    *) printf 'Usage: %s [1-6]\n' "$0" >&2; exit 2 ;;
esac
profile=$(mktemp -d "${TMPDIR:-/tmp}/nova-ledger-preview.XXXXXX")
trap 'rm -rf -- "$profile"' EXIT
mkdir -p "$profile/cache/mods" "$profile/config/nova-protocol"
cp -R webmods/the-ledger "$profile/cache/mods/the-ledger"
printf '%s\n' \
    '[(id:"the-ledger",version:"2.0.0",bundle:"the-ledger.bundle.ron")]' \
    > "$profile/cache/installed.mods.ron"
printf '%s\n' '["base","the-ledger"]' \
    > "$profile/config/nova-protocol/enabled_mods.ron"
export NOVA_MODDING_CACHE_ROOT="$profile/cache"
export NOVA_CONFIG_ROOT="$profile/config/nova-protocol"
export XDG_CONFIG_HOME="$profile/config"
export XDG_DATA_HOME="$profile/data"
export BEVY_ASSET_ROOT="$PWD"
cargo run --features dev -- --scenario "$scenario"
