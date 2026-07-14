# Persist EnabledMods cross-platform: native config file + wasm localStorage; load at startup, save on toggle

- STATUS: OPEN
- PRIORITY: 54
- TAGS: modding, wasm, spike

Spike: tasks/20260714-174000/SPIKE.md
Depends on: 20260714-174120 (EnabledMods resource) and 20260714-174126 (toggle UI).

Goal: make the `EnabledMods` set survive restarts on BOTH native and wasm. Add a small
hand-rolled cross-platform config store (native: a RON file under
`dirs::config_dir()/nova-protocol/`; wasm: `window.localStorage` via `web-sys`, behind
`#[cfg(target_arch = "wasm32")]`). Load the enabled set at startup (default: base
enabled) BEFORE `register_bundles` merges, and save it whenever a menu toggle changes
`EnabledMods`. No third-party persistence crate (Bevy 0.19 is bleeding-edge; own the two
short impls). Settle the config dir name / localStorage key / RON schema in the plan.
`spike` until planned.