# Bug: Tab drawer must render on top of the flight HUD (z-order)

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.9.0,bug,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Goal

Playtest feedback (owner, 2026-07-24): when the Tab ship-computer drawer opens
it must render ON TOP of everything else. Right now the flight HUD - notably the
compact top-right objectives panel text - still draws over the drawer panel, so
the drawer looks like it is behind the HUD instead of a modal surface above it.

This is a shell/z-order concern from 20260724-102304 (the drawer shell), NOT the
diegetic-objectives task (20260721-211520).

Scope (direction-level; /plan breaks into steps at pickup):

- Give the drawer backdrop + panel a global stacking context above all flight
  HUD widgets (Bevy 0.19: `GlobalZIndex` on the drawer root/backdrop; a plain
  `ZIndex` only reorders within one stacking context, which is why the top-right
  objectives panel currently wins). Pick a z tier above the HUD chrome.
- Verify the backdrop dims the whole HUD and the panel sits above the compact
  objectives panel, comms panel, markers, readouts - the drawer is a modal.
- The tab handle's z can stay with the HUD (it is chrome); only the OPEN
  surface must rise above.

## Steps

- [x] Write the z-order test FIRST (mirror nova_menu's overlay-z assertion at
      `crates/nova_menu/src/lib.rs:4939`): in `hud/drawer.rs` tests, register the
      `setup_drawer` observer, spawn an entity with `SpaceshipRootMarker` +
      `PlayerSpaceshipMarker` (triggers the observer), `update`, then assert the
      `DrawerRootMarker` (panel) AND `DrawerBackdropMarker` entities each carry a
      `GlobalZIndex` with value > 0, and the panel's z >= the backdrop's. Watch it
      fail (no `GlobalZIndex` today).
- [x] Add `GlobalZIndex` to the backdrop and panel in `setup_drawer`
      (`hud/drawer.rs`): backdrop `GlobalZIndex(10)`, panel `GlobalZIndex(11)` -
      the same modal tier the pause overlay uses (`nova_menu/src/lib.rs:457,540`).
      The drawer and pause menu are mutually exclusive `PauseStates` variants, so
      sharing the tier is fine. Leave the tab HANDLE at the HUD z (no
      `GlobalZIndex`) - only the OPEN surface rises above.
- [x] Verify: `cargo check --all-targets`, `cargo fmt`, the new test + the
      existing `drawer::` tests. (No probe: a UI stacking change touches no
      gameplay logic/invariants.)

## Definition of Done

- The open drawer's panel and backdrop carry an explicit `GlobalZIndex` above the
  HUD chrome (test: `drawer_renders_above_the_hud`; fails before the fix, when the
  entities carry no `GlobalZIndex`).
- manual: the owner opens the drawer in a real run and the panel sits ON TOP of
  the compact top-right objectives panel and the rest of the flight HUD - the
  reported "HUD draws over the drawer" bug is gone; the transparency + slide still
  read well.
- `cargo check --all-targets` + `cargo fmt` clean, tests green.

## Notes

- From the drawer shell (20260724-102304, LANDED c13143d4). Files:
  crates/nova_gameplay/src/hud/drawer.rs (setup_drawer spawns backdrop + panel),
  crates/nova_gameplay/src/hud/mod.rs (HUD widget spawn order). The drawer parts
  intentionally carry NO HudTier (modal axis); z-order is the remaining gap.
- Owner also confirmed (2026-07-24) they like the drawer transparency + slide
  animation - keep those.
- No CHANGELOG entry: this is pre-release polish of the still-Unreleased drawer
  feature, not a fix to a shipped release.
- Grounded (2026-07-24): `GlobalZIndex` is the Bevy 0.19 global stacking-context
  component; modal overlays in nova_menu use it (pause 10/11, outcome, tested at
  :4941/:5501). Flight HUD widgets carry none (implicit 0).

## Close-out (2026-07-24)

Added `GlobalZIndex(10)` to the drawer backdrop and `GlobalZIndex(11)` to the
panel in `setup_drawer` (`hud/drawer.rs`), plus the `DRAWER_BACKDROP_Z` /
`DRAWER_PANEL_Z` constants. The open drawer now sits in a global stacking context
above the flight HUD (which carries no `GlobalZIndex` = 0), so the compact
top-right objectives panel and other HUD no longer draw over it. The tab handle
keeps the HUD z (chrome). Same modal tier as nova_menu's pause overlay; the two
are mutually exclusive `PauseStates` variants so they never coexist.

Test `drawer_renders_above_the_hud` triggers `setup_drawer` (spawns a
`SpaceshipRootMarker`+`PlayerSpaceshipMarker` entity) and asserts the panel +
backdrop carry `GlobalZIndex > 0` with panel >= backdrop - fail-first by
construction (before the fix the entities had no `GlobalZIndex`, so the
`.single().expect(...)` on the empty query panics; mirrors nova_menu's overlay-z
assertion). Verified: 5 drawer tests green, `cargo check --workspace
--all-targets` + `cargo fmt` clean. No probe (UI stacking touches no gameplay
logic/invariants). The `manual:` "drawer above the HUD in a real run" item is the
owner's re-playtest - batched for acceptance.

Self-reflection: a small, well-scoped fix; copying nova_menu's overlay-z pattern
and its test shape (`reuse-known-good-stack`) made it near mechanical. A headless
test can only pin the z-index CONTRACT, not the actual render stacking - the
owner's eyeball is the real proof, correctly left as the manual DoD.
