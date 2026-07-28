# NOTES - Preload static assets + phosphor boot loading screen

## New-entry sweep for `GameAssetsStates::Boot` (Step 1, per `audit-state-gates-on-new-entry-path`)

Adding `Boot` as the new `#[default]` before `Loading` changes the app's FIRST
state. Swept every `GameAssetsStates` reference across the whole workspace
(`grep -rn GameAssetsStates crates/ examples/`) for what newly runs / breaks:

- Every EXTERNAL consumer hooks `OnEnter(GameAssetsStates::Loaded)` only
  (all screenshot/section/gameplay examples, `nova_core`'s menu handoff,
  `nova_probe`, `nova_debug`'s harness). Loaded is still reached
  (`Boot -> Loading -> Processing -> Loaded`), just one extra hop earlier, so
  none are affected.
- The three `not(in_state(GameAssetsStates::Loading))` Update re-run gates in
  `nova_assets` (`build_mod_catalog`, `save_enabled_mods`, and the live re-merge)
  are ALSO guarded by `resource_exists::<GameAssets>`, which is false during
  `Boot` (the collection is inserted only when `Loading` finishes). So they do
  not fire during Boot - safe.
- Nothing keyed on `Loading` being the DEFAULT/first state; nothing runs during
  `Boot` except the boot loading-state (`load_collection::<BootAssets>`),
  `fill_ui_font` at `OnExit(Boot)`, and the loading screen spawned at
  `OnEnter(Loading)`.
- `BCS_SHOT` force-sets `GameStates` (not `GameAssetsStates`), so the boot chain
  still walks normally under the screenshot harness.

Conclusion: the Boot variant is additive; no gate, observer, or hook needed
widening.

## Ordering: UiFont must exist before the loading screen spawns

`fill_ui_font` runs at `OnExit(GameAssetsStates::Boot)`, which fires BEFORE
`OnEnter(GameAssetsStates::Loading)` in the same transition. The loading screen
spawns at `OnEnter(Loading)`, so `UiFont` is always present when it reads it
(themed text from the first frame). Chosen over running `fill_ui_font` in
`OnEnter(Loading)`, where ordering vs the cross-crate screen-spawn system would
be undefined.

## Loading-screen camera

Nothing else renders during load, so the screen owns a `Camera2d`. Both the
panel and the camera despawn at `OnEnter(Loaded)`; the menu/gameplay cameras
spawn on their own later `OnEnter`, so there is no lasting two-camera state.

## Verification

- `cargo check --workspace --all-targets`: clean.
- New tests pass: `boot_then_loading_collections_gate_in_sequence`,
  `ui_sfx_collection_matches_ui_sfx_files`,
  `loading_screen_spawns_in_loading_and_despawns_on_loaded`,
  `loading_dots_march_over_time`.
- DoD 4 greps: `load("icons/` and `load(NOVA_OS_FONT_PATH)` both 0 hits.
- DoD 5: `git ls-files assets/fonts` = only the `.ttf`; 10.8 MB; 0 `.ttc` refs
  outside `tasks/`.
