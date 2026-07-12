# Review: Fix ammo-count number showing outside F11/debug mode

- TASK: 20260712-173928
- BRANCH: fix/ammo-number-debug-desync

## Round 1

- VERDICT: APPROVE

Debug-only one-line wiring fix, self-reviewed with the root cause re-derived from
source: nova_debug's `toggle_debug_mode` is ungated (lib.rs:45) while
`toggle_ammo_readout_debug` was gated `in_state(Playing)` - an F11 outside Playing
desynced the two `true`-default mirrors. Removing the gate makes them flip
together on every F11, so the number is visible iff debug mode is on. Global
`DebugEnabled(true)` default untouched (out of scope; at boot both are on =
consistent). Regression test asserts the flip; existing visibility test still
passes. Checks: check --all-targets clean, ammo_readout 11/11 (--features debug),
fmt clean. No findings.
