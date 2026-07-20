# Review: Mods main-menu section (list + enable/disable, base locked)

- TASK: 20260714-174126
- BRANCH: menu/mods-panel

## Round 1

- VERDICT: APPROVE

Reviewed the diff (nova_assets `ModCatalog` + `build_mod_catalog` + prelude re-exports;
nova_menu Mods button + modal `ModsPanel` + `on_mod_toggle`/`update_mod_toggle_labels`/
`scroll_mods_panel`). Self pass plus an independent out-of-context adversarial pass.
Independently verified the goal-critical cross-crate chain:

- Toggle -> EnabledMods -> live re-merge: `on_mod_toggle` reads `activate.entity` (the
  bevy_ui_widgets button sets `Activate.entity` to the button entity, which is where
  `observe(on_mod_toggle)` is attached) and mutates `ResMut<EnabledMods>` (insert/remove),
  marking it changed, so nova_assets' `register_bundles.run_if(resource_changed::<EnabledMods>)`
  re-runs. Same resource instance in one World (GameAssetsPlugin inits it, the menu mutates
  it). 174120's `toggling_enabled_mods_remerges_live` already proved a mutation re-merges.
- Re-merging in MainMenu updates `GameScenarios` (the config map) for the next New Game
  without disturbing the already-spawned menu ambience scene - matches the spike's intent.
- base is locked twice over: no toggle button spawned for base rows, plus the
  `if toggle.base { return; }` guard.

Verification: nova_menu (11 tests, +3 new) and nova_assets (demo_scenario 6, +1) pass;
`cargo test --workspace --no-run` green; fmt clean; `12_menu_newgame` runs clean headless
(0 loader errors, no panic) with the Mods panel spawned.

The out-of-context reviewer concurred (no blockers/majors) and confirmed:
`scroll_mods_panel`'s `resource_exists::<Messages<MouseWheel>>` guard is correct (real
app's InputPlugin provides it; headless tests skip it, avoiding a MessageReader panic);
`update_mod_toggle_labels` touches only the button's Text child (no fight with
`update_button_colors`, which owns BackgroundColor); `setup_menu_ui`'s `Option` guarding
builds cleanly with no catalog; `ModsPanel` carries `DespawnOnExit(MainMenu)` (no
cross-state leak); the tests are non-tautological (would fail if the feature broke).

- [ ] R1.1 (MINOR) [pre-existing - deferred] The Settings and Mods full-screen modals can
  both be Visible at once and overlap, since each button independently toggles its own
  panel's Visibility. This is the SAME behavior the Settings panel already had before this
  task (not a regression), and it is a UX polish item, not a correctness bug. Left as-is to
  keep scope tight; worth a small follow-up (mutually-exclusive modal visibility) if the
  menu grows more panels.

No BLOCKER/MAJOR. Ships.
