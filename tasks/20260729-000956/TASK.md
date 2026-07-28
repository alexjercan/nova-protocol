# Preload static assets via bevy_asset_loader + phosphor boot loading screen

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.9.0,ui,assets

## Story

Owner directive (2026-07-28, /flow): static assets should PRELOAD through
bevy_asset_loader collections during the startup loading phase, not lazy
`asset_server.load_*` at spawn time - and startup gets a small LOADING
screen in the phosphor look. This is the verification/alignment pass for
epic 20260728-175719: fix the collection bypasses already in the code,
re-point the epic's still-open child plans (175734, 175742) at the
collection pattern before they implement, and add the loading screen.

Audit findings (2026-07-28 code map):

- The gate already exists: `nova_assets` drives `GameAssetsStates`
  Loading -> Processing -> Loaded via bevy_asset_loader with the
  `GameAssets` collection (crates/nova_assets/src/lib.rs:1050-1072).
  Nothing renders during Loading - no camera or UI until the
  OnEnter(Loaded) handoff (crates/nova_core/src/lib.rs:178); native shows
  a blank window, web shows the HTML lds-dual-ring spinner (index.html).
- Bypasses today: the NOVA OS font `server.load(NOVA_OS_FONT_PATH)` at
  crates/nova_gameplay/src/hud/nova_os.rs:2553 - a 66 MB family .ttc
  (assets/fonts/SGr-IosevkaTerm-Regular.ttc) fetched LAZILY when the
  drawer first spawns (on the shipped web build that is a 66 MB download
  on first Tab press); `icons/nova_crt_mark.png` via `asset_server.load`
  at nova_os.rs:3683 and hud/objective_hint.rs:117; the 15 UI SFX wavs
  via `SoundBank::load` at OnEnter(Processing)
  (nova_assets/src/lib.rs:1206) - kicked early but never load-gated.
- Queued bypasses in the open child plans: 175742 plans a
  `KeyCode -> asset path` lookup for the key glyphs (implies per-chip
  `server.load`; the CLOSED 233707's Notes prescribe exactly that -
  append-only history now, the correction lands in the consumer);
  175734 loads Iosevka "into a nova_ui font resource at startup" with
  the mechanism unspecified.
- Legit dynamic loads that stay on the asset server BY DESIGN: scenario
  content `AssetRef`s (nova_gameplay/src/asset_ref.rs:98,
  content-authored paths), downloaded mods via `mods://`
  (nova_assets/src/lib.rs:456, portal.rs:1300). Runtime-varying, cannot
  sit behind a one-shot collection; this is the "when possible" boundary.

## Steps

- [x] Boot state: add `GameAssetsStates::Boot` as the new `#[default]`
      variant before Loading in crates/nova_assets/src/lib.rs, with a
      second loading state
      (`LoadingState::new(Boot).continue_to_state(Loading)
      .load_collection::<BootAssets>()`); `BootAssets` holds the UI font.
      Verify-first: bevy_asset_loader keys its internal schedules per
      state VALUE (loading_state.rs:497, `OnEnterInternalLoadingState(S)`)
      so two loading states on one enum should chain - pin it with the
      DoD 1 test before building on it. New-entry sweep: grep everything
      gated on `GameAssetsStates` and record the result in NOTES.md (the
      three `not(in_state(Loading))` Update gates in nova_assets are safe:
      their `resource_exists::<GameAssets>` guard is false during Boot;
      examples/probe/debug harness all hook OnEnter(Loaded)).
- [x] Font payload: replace the 66 MB
      assets/fonts/SGr-IosevkaTerm-Regular.ttc with the single Regular
      face as a .ttf. Primary route (offline, keeps the exact shipped
      version): extract with fonttools
      (`nix shell nixpkgs#python3Packages.fonttools`, TTCollection ->
      save the Regular face; verify the face count/name table first).
      Alternative: the upstream per-weight TTF package for the same
      Iosevka version. Update NOVA_OS_FONT_PATH (nova_os.rs:156),
      credits/CREDITS.md:41, and every other live .ttc reference
      (repo-wide grep, tasks/ excluded).
- [x] `UiFont` resource: new nova_ui resource holding the `Handle<Font>`;
      nova_assets fills it from `BootAssets` at OnEnter(Loading) (add the
      direct nova_ui dep if missing); nova_os.rs:2553 reads it instead of
      `server.load`. This pre-builds the "nova_ui font resource" that
      175734's typography step plans, so that step becomes a consumer edit.
- [x] Loading screen: new crates/nova_core/src/loading_screen.rs
      (nova_core owns the Loaded handoff + status UI already; add its
      missing direct nova_ui dep). OnEnter(GameAssetsStates::Loading):
      spawn a 2D UI camera + full-screen phosphor panel - near-black
      screen, green phosphor Iosevka text via `UiFont` ("NOVA OS" mark +
      a LOADING line), an INDETERMINATE CRT-style animation (blinking
      block cursor + marching dots; no new progress-tracking dependency),
      amber accent per examples/ui/nova_os_terminal_poc.html. Despawn
      screen + camera at OnEnter(GameAssetsStates::Loaded). Write the
      live-tree test FIRST (mode-keyed-reconciler lesson).
- [x] Align the crt mark: add `icons/nova_crt_mark.png` to `GameAssets`,
      push the handle through `NovaHudAssets` in `update_nova_hud_assets`
      (nova_assets/src/lib.rs:1211), switch nova_os.rs:3683 +
      objective_hint.rs:117 to the resource.
- [x] Align the UI SFX: mapped audio collection on `GameAssets`
      (`collection(typed, mapped)`, explicit `paths(...)` over the 15
      `UI_SFX_FILES` paths, keyed by `AssetFileStem`) + a
      `SoundBank::from_handles` constructor; `register_sounds` builds the
      bank from the collection; parity test asserts collection paths ==
      `nova_gameplay::audio::UI_SFX_FILES` (the const is reachable -
      nova_assets already depends on nova_gameplay).
- [x] Re-point the open child plans (dated alignment notes + edits to the
      affected Steps/Notes lines, no history rewrites):
      175742 - the KeyCode mapping resolves HANDLES from a
      `collection(typed, mapped)` glyph collection with explicit
      `paths(...)` (folder collections do not work on wasm) listing the
      mapping table's used glyphs; extend its DoD test 2 to also assert
      every mapped path is in the collection; note that the
      remap/gamepad follow-up (20260710-231927) may fall back to
      `server.load` for unmapped keys (dynamic-content exception).
      175734 - the typography step consumes `UiFont` from this task
      instead of loading the font itself.
- [x] Epic 20260728-175719 TASK.md: add a Decisions pointer to this
      task's DECISION.md (the Child Tasks row was added at plan time).
- [x] Docs + CHANGELOG: one line in the nova_assets crate docs (static
      assets preload via collections; scenario AssetRefs and mods:// are
      the dynamic exceptions) and a CHANGELOG [Unreleased] line for the
      loading screen + font slimming.
- [x] Verify per DoD (screenshot eyeball for the screen; no probe run -
      startup-only change, no gameplay behavior touched).

## Definition of Done

1. test: `boot_then_loading_collections_gate_in_sequence` - headless app
   walks Boot -> Loading -> Processing -> Loaded with both collections
   resolved (fails if two loading states on one enum do not chain).
2. test: `loading_screen_spawns_in_loading_and_despawns_on_loaded` -
   live-tree test, written before the implementation.
3. test: `ui_sfx_collection_matches_ui_sfx_files` - collection/const
   parity pin.
4. cmd: `grep -rn 'load("icons/' crates/` and
   `grep -rn "load(NOVA_OS_FONT_PATH)" crates/` both print 0 hits
   (today: nova_os.rs:3683 + objective_hint.rs:117, and nova_os.rs:2553).
5. cmd: `git ls-files assets/fonts` prints only
   `assets/fonts/SGr-IosevkaTerm-Regular.ttf`, the file is under 15 MB
   (down from 66 MB), and `grep -rn "SGr-IosevkaTerm-Regular.ttc"
   --exclude-dir=tasks --exclude-dir=node_modules --exclude-dir=dist .`
   prints 0 hits.
6. cmd: `grep -n "collection" tasks/20260728-175742/TASK.md` and
   `grep -n "UiFont" tasks/20260728-175734/TASK.md` hit (open child
   plans re-pointed).
7. manual: native run shows the phosphor LOADING screen at startup, then
   hands off to the menu (screenshot eyeball; web spinner still covers
   the pre-wasm + Boot phase).

## Notes

- Fork decisions to confirm at the plan gate (DECISION.md on approval):
  (a) Boot-state font gating for the screen vs bevy's `default_font`
  feature vs a font-free screen - Boot collection recommended (themed
  text from the first frame, stays inside the bevy_asset_loader model);
  (b) single-face .ttf swap vs keeping the 66 MB .ttc vs glyph-level
  subsetting - face extraction recommended, subsetting deferred (revisit
  in 175734 if the web payload still hurts; subsetting risks dropping
  box-drawing/arrow glyphs the NOVA OS output uses);
  (c) indeterminate CRT animation vs iyes_progress percent bar - the
  percent bar needs a new dependency + per-collection wiring; deferred.
- Processing is effectively one frame (the OnEnter chain sets Loaded
  itself, nova_assets/src/lib.rs:1057-1072), so the screen mostly covers
  Loading; despawning at OnEnter(Loaded) leaves the menu's own spawn to
  the existing GameStates handoff. Examples built with
  `with_main_menu(false)` get the same screen until Loaded - harmless,
  and BCS_SHOT force-sets GameStates, not GameAssetsStates.
- The glyph collection itself lands with 175742 (its mapping table owns
  the path list); this task only aligns the plan text. 233707 landed
  mid-planning (glyphs already at assets/input-prompts/keyboard/Alt/),
  so its CLOSED record keeps its server.load note as history.
- Depends on: nothing open. Slots BEFORE 175734/175738/175742/175747
  (p50 vs p40/38/36/34) so the pattern exists before its consumers.

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED
