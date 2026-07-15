# Review: Hidden dev mods - catalog hidden flag keeps screenshot-reel out of the Mods menu

- TASK: 20260715-142844
- BRANCH: feature/hidden-dev-mods

## Round 1

- VERDICT: APPROVE (one MINOR, addressed before landing)

Out-of-context review pass (fresh-context agent over the full diff vs master;
all close-out claims re-verified independently). Verified: single focused
commit matching Steps 1-7; ModCatalog's only consumer is nova_menu, so
`build_mod_catalog` is the right single choke point; `seed_enabled_mods` /
`register_bundles` read the full `InstalledCatalog`, keeping hidden orthogonal
to enabled; the example enables by id at `OnEnter(Loaded)`, untouched;
`mod_catalog_lists_installed_mods_metadata` fails with the filter deleted
(master CI on 92aaf8da is the live fail-first proof); the extended decode test
cannot compile without the field; docs and close-out match behavior; reel
content ids (`reel_*`, `screenshot_reel`) cannot collide with base. Test
re-runs: nova_modding 3 passed, demo_scenario 8 passed, nova_menu 13 passed;
fmt + check --workspace --all-targets clean.

- [x] R1.1 (MINOR) crates/nova_assets/src/lib.rs:183-196 - a run of
  `examples/13_screenshot_reel.rs` persists `screenshot-reel` into the shared
  prefs store via `save_enabled_mods`; a later normal run restores it and
  merges it, and with this change there is no menu row to see or disable the
  stuck-enabled hidden mod (pre-change it was visible and toggleable). Dormant
  today (reel ids collide with nothing and the scenario loads only by explicit
  id), but the state is unreachable-to-fix from the UI. Suggested change: strip
  hidden ids from `EnabledMods` in the `OnEnter(Processing)` chain (catalog is
  guaranteed loaded there), making hidden-mod enablement session-only; the
  example is unaffected (it re-inserts at `OnEnter(Loaded)`, after the chain,
  each run - its doc comment says it does this deliberately to survive prefs
  restoration).
  - Response: fixed in the follow-up commit on this branch - the strip lives in
    `seed_enabled_mods` (already catalog-aware, already rig-tested), which now
    RECONCILES `EnabledMods` with the catalog: unions `base: true` ids in AND
    strips `hidden && !base` ids out (the `!base` guard keeps a pathological
    hidden+base entry force-enabled). Restored prefs shed hidden ids at every
    startup, and the same-frame change makes `save_enabled_mods` rewrite the
    cleaned set, so a polluted prefs file self-heals on the next normal launch.
    Session-only semantics documented in the system doc and
    docs/modding-ron-format.md. New rig case: `seed_from(["demo",
    "screenshot-reel"])` yields base+demo without screenshot-reel (fails with
    the strip deleted). Verified by reviewer: implementation matches the
    suggestion, the new assertion is the strip's boundary pin, and the example
    path is untouched (`OnEnter(Loaded)` insert happens after the strip).
