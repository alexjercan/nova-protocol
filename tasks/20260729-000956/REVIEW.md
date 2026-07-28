# REVIEW - Preload static assets + phosphor boot loading screen

## Round 1 - out-of-context reviewer (2026-07-29)

Independent reviewer read the full `feat/preload-assets` diff + new files, focused
on correctness and blast radius (asset pipeline + web build).

### Findings

- **MINOR** `crates/nova_ui/src/font.rs` docstring: claimed `UiFont` is filled at
  `OnEnter(GameAssetsStates::Loading)` but the implementation runs `fill_ui_font`
  at `OnExit(GameAssetsStates::Boot)`. Practically equivalent (OnExit(Boot) runs
  before OnEnter(Loading)), but the comment was inaccurate.
  - FIXED: docstring now says `OnExit(Boot)` (which runs before
    `OnEnter(Loading)`).

### Areas reviewed - no issues

- Two-loading-state chain + `OnExit(Boot)` `UiFont` fill lifecycle: correct;
  `UiFont` guaranteed present before the loading screen spawns.
- UI-SFX route-around: load-gating achieved; `SoundBank::load` and the mapped
  collection reference the SAME assets (AssetServer dedups by path) - no
  double-load. The 15 `UI_SFX_FILES` entries -> 14 distinct files (CommsLine
  reuses `ui_toggle`) match `UI_SFX_COLLECTION_PATHS`.
- `.ttc` -> `.ttf` swap + `NovaOsTtcFontLoader` removal: complete; `nova_meta_gen`
  correctly registers `bevy::text::FontLoader`; web `.meta` sidecar path intact;
  0 remaining `.ttc`/loader refs.
- crt-mark move to `NovaHudAssets`: headless rigs still spawn the plate/hint
  without the logo (guard moved from AssetServer presence to NovaHudAssets
  presence).
- Boot-default new-entry sweep: the three `not(in_state(Loading))` gates are
  `resource_exists::<GameAssets>`-guarded (false during Boot); every external
  consumer hooks `OnEnter(Loaded)`. Safe.
- Test quality: all four new tests fail if their mechanism is a no-op
  (live-tree entity counts, real async loads with timeout, observed dot
  animation, path/disk parity).

### Verdict

APPROVE (one MINOR, addressed on-branch).
