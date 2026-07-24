# Status bar gets its own HudTier::Status (persists through drawer + Minimal, clears at cinematic None)

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.9.0,feature,ui,hud

# Status bar gets its own HudTier::Status (persists through the drawer + Minimal; clears only at cinematic None)

## Goal

Playtest (owner, 2026-07-24): opening the Tab drawer hides the top-right status
bar (the bcs `status_bar`: fps + version + the objective count). The status bar
is reference/overlay chrome ("like the FPS overlay") and should NOT be governed
by the gameplay HUD-hide the way flight instruments and learning cues are - it
should persist while you fly and while the drawer is open, and vanish only when
you deliberately clear the screen for a cinematic capture.

## Design (owner gate 2026-07-24; see DECISION.md)

Add a THIRD `HudTier::Status` for persistent status/reference chrome:
- `HudVisibility::shows`: Status is visible at `All` AND `Minimal`, hidden at
  `None` (the cinematic clean-screen level) - so screenshots stay clean.
- The status bar persists through the DRAWER via the existing `HudDrawerExempt`
  mechanism (which also z-lifts it above the drawer backdrop). Retag the
  `status_bar` root in `nova_core::setup_status_ui`: `Chrome` -> `Status`, and
  add `HudDrawerExempt` + `GlobalZIndex::default()`.
- The objective count (a child of the status bar root) inherits this, so it
  persists too.
- SCOPE: only the top-right status bar. The top-center readout strip already
  behaves correctly (Instrument + HudDrawerExempt) and is left as-is; a later
  migration to `Status` for semantic consistency is noted in DECISION.md, not
  done here.

## Steps

1. Add `HudTier::Status` variant (hud/mod.rs) with a doc comment defining it as
   persistent reference chrome. Update `HudVisibility::shows` so `Minimal` shows
   `Status` (like `Instrument`); `None` still shows nothing.
2. Retag the status bar (nova_core/src/lib.rs `setup_status_ui`): `HudTier::Chrome`
   -> `HudTier::Status`, add `HudDrawerExempt` + `GlobalZIndex::default()`.
3. Tests (hud/mod.rs): `Status` shows at All + Minimal, hides at None; a
   `Status`+`HudDrawerExempt` widget survives the drawer (extend the existing
   apply_hud_visibility tests).
4. Verify: `cargo check -p nova_gameplay` + `-p nova_core`, `cargo fmt --check`,
   the new tests, and a `nova_probe` playable run.

## Definition of Done

1. Opening the drawer leaves the top-right status bar (fps/version/objective
   count) visible and readable (manual: owner opens the drawer in-game).
2. Grave/tilde `Minimal` keeps the status bar; `None` still clears it for a
   clean screenshot (test: `shows` for Status at each level; manual: cinematic
   None is clean).
3. `cargo check` (nova_gameplay + nova_core) + `cargo fmt --check` clean (cmd);
   probe playable OK (probe).
