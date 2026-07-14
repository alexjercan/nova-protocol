# Persist EnabledMods cross-platform: native config file + wasm localStorage; load at startup, save on toggle

- STATUS: CLOSED
- PRIORITY: 54
- TAGS: modding, wasm

Spike: tasks/20260714-174000/SPIKE.md
Depends on: 20260714-174120 (EnabledMods resource) and 20260714-174126 (toggle UI).

Goal: make the `EnabledMods` set survive restarts on BOTH native and wasm. Add a small
hand-rolled cross-platform config store (native: a RON file under
`dirs::config_dir()/nova-protocol/`; wasm: `window.localStorage` via `web-sys`, behind
`#[cfg(target_arch = "wasm32")]`). Load the enabled set at startup (default: base
enabled) BEFORE `register_bundles` merges, and save it whenever a menu toggle changes
`EnabledMods`. No third-party persistence crate (Bevy 0.19 is bleeding-edge; own the two
short impls).

## Plan (20260714)

Store: the enabled-mod ids as a RON `Vec<String>`. Native path
`dirs::config_dir()/nova-protocol/enabled_mods.ron`; wasm `localStorage` key
`nova_protocol.enabled_mods`. Deps already in the lockfile: `dirs` 6, `web-sys` 0.3
(target-gated). NOTE: the wasm target is NOT installed locally, so the wasm backend can
only be reviewed statically here (CI/Trunk builds it); keep it minimal + behind cfg.

Robustness change to `seed_enabled_mods`: switch from "seed only if empty" to "always
UNION the catalog's `base:true` ids into `EnabledMods`". This keeps base enabled
regardless of what was loaded (base is locked in the UI) AND preserves the persisted
non-base choices - cleaner than the only-if-empty guard and makes load-order-independent.

Steps:
- [x] 1. nova_assets Cargo.toml: `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
  dirs = "6"`; `[target.'cfg(target_arch = "wasm32")'.dependencies] web-sys = { version =
  "0.3", features = ["Window", "Storage"] }`.
- [x] 2. nova_assets `mod_prefs` module: cross-platform `load_enabled_ids() ->
  Option<Vec<String>>` + `save_enabled_ids(ids: &[String])`, cfg-split. Native: read/write
  the RON file (factor the IO into pure `load_from(path)` / `save_to(path, ids)` so it is
  unit-testable with a tempfile; `save` creates the parent dir). Wasm: `web_sys::window()
  .local_storage()` get/set the key. Both tolerate missing/corrupt data (return None / no-op).
  `None` = "no saved prefs yet" (fall back to defaults); `Some` = authoritative.
- [x] 3. nova_assets: `load_enabled_mods` system at the START of the `OnEnter(Processing)`
  chain (before `seed_enabled_mods`): if `load_enabled_ids()` is `Some`, set `EnabledMods`
  from it. `save_enabled_mods` system in `Update`, `run_if(resource_changed::<EnabledMods>)`:
  call `save_enabled_ids`. (Startup writes the default set once; menu toggles write updates -
  acceptable, IO is tiny.)
- [x] 4. nova_assets: change `seed_enabled_mods` to UNION base ids (per above) instead of
  only-if-empty. Update its doc + the 174120 test expectations if needed.
- [x] 5. Tests (nova_assets): (a) native `save_to`/`load_from` round-trip via a tempfile
  (write ids, read them back; a missing file -> None; corrupt bytes -> None); (b)
  `seed_enabled_mods` unions base: `EnabledMods` empty -> `{base}`, `{demo}` -> `{base,demo}`,
  driven with the real catalog. (wasm localStorage path is cfg-guarded and reviewed
  statically - note it in the test module.)
- [x] 6. Verify: `cargo test --workspace --no-run`; nova_assets tests; `cargo fmt`;
  `12_menu_newgame` headless runs clean; a native manual/e2e check: with a temp
  `XDG_CONFIG_HOME`, the `enabled_mods.ron` file is written on a toggle and re-read on the
  next launch (or assert `save_enabled_ids`+`load_enabled_ids` round-trip against the real
  path helper under a temp `XDG_CONFIG_HOME`). Confirm behaviour with NO saved file is the
  base-only default (unchanged startup).

This is the LAST task of the mod-manager flow (spike 174000): after it, enabling the demo
mod from the menu persists across restarts on native and web.