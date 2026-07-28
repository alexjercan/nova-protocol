# RETRO - Preload static assets + phosphor boot loading screen

Task 20260729-000956 (epic 20260728-175719). Landed as one squash commit.

## What changed and why

Static assets now PRELOAD through `bevy_asset_loader` collections and are
load-gated before gameplay, instead of lazy `asset_server.load` at spawn time:

- A second loading state on `GameAssetsStates` (`Boot -> Loading`) loads a tiny
  `BootAssets` collection (the UI font), published as the new
  `nova_ui::font::UiFont` resource at `OnExit(Boot)`. This gives the new phosphor
  boot loading screen (`nova_core::loading_screen`) its themed Iosevka face from
  the first `Loading` frame.
- The 66 MB Iosevka `.ttc` was replaced by the single Regular face as a 10.8 MB
  `.ttf` (fonttools face extraction, all glyphs kept). Bevy's built-in
  `FontLoader` claims `.ttf`, so the bespoke `NovaOsTtcFontLoader` was retired
  (game + `nova_meta_gen`). This kills a 66 MB first-Tab download on the web.
- The NOVA CRT brand mark and the 15 UI SFX joined `GameAssets` (crt via
  `NovaHudAssets`, SFX via a `collection(mapped, typed)` field), removing the
  last lazy `load("icons/...")` and load-gating the SFX.

## Key decisions (see DECISION.md)

- D4 - UI SFX WITHOUT a bcs release: the planned `SoundBank::from_handles` turned
  out to already exist in `bevy_common_systems` but PRIVATE, behind a git TAG.
  Exposing it means releasing a separate repo (outward-facing), which the owner
  had not asked for. Route-around: the mapped collection provides the load-gate;
  `register_sounds` keeps the public `SoundBank::load` (AssetServer dedups by
  path, so both reference the same gated assets). Parity pinned by
  `ui_sfx_collection_matches_ui_sfx_files`.
- D5 - DoD-1 test altitude: the full `GameAssets` walk (glTF `WorldAsset`s,
  audio, mod catalog) has no headless loader set in the test suite - it is only
  exercised by the render-based screenshot harness. So the DoD-1 test pins the
  ONLY novel mechanism (two loading states chaining on one enum) on the real
  enum with lightweight real image loads, not the shipped heavy collections.

## Difficulties + how they were handled

- The bespoke `.ttc` font loader was an easy surface to MISS: nothing in the
  task text named it. Grepping for `.ttc` (not just the font path) surfaced
  `NovaOsTtcFontLoader` + its `nova_meta_gen` registration + its test - a whole
  web-build sub-surface that would have broken (`AssetMetaCheck::Always` needs a
  `.meta` sidecar; the wrong loader = no sidecar = invisible glyphs, the exact
  bug 20260727-172205 fixed). Removing it and repointing to `bevy::text::FontLoader`
  was the fix.
- `AssetApp` became a test-only import once the custom loader's
  `register_asset_loader` was removed (the lib no longer used it, but `#[cfg(test)]`
  rigs call `init_asset`). Gated the import `#[cfg(test)]` to keep the lib
  warning-free.
- Ordering: filling `UiFont` in `OnEnter(Loading)` alongside the cross-crate
  loading-screen spawn is order-undefined. Moved the fill to `OnExit(Boot)` so it
  provably precedes the screen.

## Self-reflected feedback (for future sessions)

- On any font/asset-format change, grep the FORMAT EXTENSION and the loader
  types, not just the asset path const. The `.ttc`-specific custom loader and
  its meta-generator twin were invisible to a path-only sweep.
- When a plan step names a specific API (`SoundBank::from_handles`), verify that
  API's real visibility/availability BEFORE committing to it - a private method
  behind a tagged external dep changes the whole approach (here: route around it
  rather than force a cross-repo release), and that fork belongs in a DECISION.md.
- Heavy integration tests (full `GameAssets` load) are not the established
  pattern here for a reason (no headless loader set; the screenshot harness owns
  that altitude). Pin the NOVEL mechanism at the lightest faithful altitude and
  say so, rather than writing a fragile full-load unit test.

## Verification

- `cargo check --workspace --all-targets` clean; `cargo fmt --check` clean.
- New tests pass: `boot_then_loading_collections_gate_in_sequence`,
  `ui_sfx_collection_matches_ui_sfx_files`,
  `loading_screen_spawns_in_loading_and_despawns_on_loaded`,
  `loading_dots_march_over_time`; `nova_meta_gen` `generate` test passes with the
  built-in FontLoader.
- DoD 4 (0 lazy icon/font loads), DoD 5 (only the `.ttf`, 10.8 MB, 0 `.ttc`
  refs), DoD 6 (child plans re-pointed) verified by grep.
- Review round 1 (out-of-context): APPROVE, one MINOR doc fix addressed.
- DoD 7 (visual eyeball of the loading screen) is a `manual:` owner-acceptance
  item; the live-tree test is the automated proof.
