# HUD visibility levels: tilde cycles ALL/MINIMAL/NONE

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.5.0,hud,ui,spike

## Goal

Let the player hide UI chrome for cinematic shots. A
`HudVisibility { All, Minimal, None }` resource; pressing grave/tilde
cycles All -> Minimal -> None -> All. Chrome (hints, brackets, edge
arrows, objectives, status bar) survives only at All; flight/combat
instruments survive Minimal; None hides everything.

## Steps

- [x] Add `HudVisibility { #[default] All, Minimal, None }` as a Resource
      in `crates/nova_gameplay/src/hud/mod.rs`, init + register_type in the
      HUD plugin, export via the hud prelude. Add a `HudTier { Instrument,
      Chrome }` component for module roots (Instrument = visible at
      All+Minimal, Chrome = All only).
- [x] Cycle input: a small Update system in the HUD plugin using plain
      `ButtonInput<KeyCode>` `just_pressed` (pattern: nova_debug F11 toggle,
      crates/nova_debug/src/lib.rs:68), gated to GameStates::Playing so the
      menu is unaffected. Grep confirmed no existing Backquote/Grave binding
      anywhere; confirm the exact Bevy 0.19 KeyCode variant name
      (Backquote) when coding.
- [x] Tag every HUD module root with its HudTier where it spawns
      (hud/mod.rs:85-102 observers): Instrument = velocity spheres
      (VelocityHudMarker), flight status chips, maneuver instruments,
      torpedo target reticle, turret lead pips, health display; Chrome =
      keybind hints (cluster + verb cues), edge indicators, target
      candidates, component lock markers, objectives panel. Status bar
      (StatusBarRootMarker, spawned in nova_core) is Chrome - tag or treat
      it specially in the apply system.
- [x] Central apply system: on HudVisibility change (and every frame for
      ephemeral entities, see next step), write Visibility (Hidden vs
      Inherited) on all HudTier roots per the current level. VERIFY FIRST
      how the screen_indicator widget drives node Visibility in PostUpdate
      (crates/nova_gameplay/src/hud/screen_indicator.rs:195): if it writes
      Visibility::Visible on indicator nodes, a hidden ancestor does NOT
      hide them (Visible ignores the parent) - in that case either order
      the apply after the projection in PostUpdate and overwrite, or teach
      the widget to respect HudVisibility directly. Do not default to the
      Update HUD set (LESSONS: consumer slots after producer schedule).
- [x] Ephemeral world-space instruments: trajectory ribbon segments and
      flip gate (holo_instruments.rs sync systems spawn/despawn per leg)
      and the maneuver orbit ring / radius spoke meshes are (re)spawned at
      runtime - verify whether gating their sync systems with
      run_if(HudVisibility != None-for-their-tier) leaves stale segments
      behind; if so, also write Visibility on their markers each frame or
      despawn on level change. Pick the mechanism against the code.
- [x] Absorb the menu's status-bar hide: nova_menu OnEnter(MainMenu) sets
      HudVisibility::None and OnExit restores All (replacing
      hide_status_bar/show_status_bar), with the apply system owning the
      status bar. Note: a mid-game cycle level then menu round-trip resets
      to All - acceptable, document it.
- [x] Add a `~ HUD` row to the keybind hints cluster (keybind_hints.rs) so
      the cycle is discoverable (the row itself is Chrome and disappears
      with the hints).
- [x] Tests in nova_gameplay hud module (headless App): (a) cycle logic
      All -> Minimal -> None -> All on repeated presses (delivery guard:
      assert the press was registered each step, LESSONS
      assert-each-gesture-step); (b) apply semantics: spawn one Instrument
      root and one Chrome root, step through the three levels, assert
      Visibility Hidden/Inherited per tier at every level; (c) whatever the
      screen-indicator verify decides gets its own regression test.
- [x] Run check/fmt + the new tests; verify visually under Xvfb (New Game,
      press tilde twice, screenshots at each level) - delivery guard: the
      All screenshot must show the chrome the Minimal one lacks.
- [x] Docs: CHANGELOG entry; short section in docs/architecture.md or the
      hud module doc on HudVisibility/HudTier; Fix record line in
      tasks/20260711-180500/SPIKE.md; update the nova_menu
      comment that pointed at this task.

## Notes

- Spike: tasks/20260711-180500/SPIKE.md (gesture choice: plain
  press-to-cycle, no hold).
- Parent task: 20260711-174915.
- Research inventory (2026-07-11 session): all HUD modules spawn via
  observers on PlayerSpaceshipMarker with root markers (hud/mod.rs:85-102);
  NovaHudSystems covers module update systems but stopping systems does NOT
  hide already-spawned UI nodes; screen_indicator projection runs in
  PostUpdate outside NovaHudSystems; juice.rs gizmo flashes are FX, not
  HUD - deliberately unaffected by this task.
- Toggle precedents: DebugEnabled resource_equals run_if
  (nova_debug/src/lib.rs:59), status bar Visibility write
  (nova_menu/src/lib.rs hide_status_bar).
- No existing grave/tilde binding conflicts (repo-wide grep).

## Close record (2026-07-11)

What changed:
- HudVisibility { All, Minimal, None } resource + HudTier { Instrument,
  Chrome } component in crates/nova_gameplay/src/hud/mod.rs; grave/tilde
  (KeyCode::Backquote, no existing binding conflicts) cycles the level via
  plain ButtonInput, gated to GameStates::Playing.
- Enforcement (apply_hud_visibility) runs in PostUpdate AFTER
  ScreenIndicatorSystems - the verify-first step confirmed the widget
  writes Visibility::Visible on its nodes every frame (Visible ignores
  hidden ancestors), so tier-hidden indicator nodes are overwritten
  downstream every frame, resolving tier from the nearest tagged ancestor;
  plain roots get per-frame Hidden plus a one-shot Inherited restore on
  level change (self-driving widgets like the gravity sphere re-assert
  their own state).
- All module roots tagged at spawn (Instrument: velocity spheres, flight
  status, destination marker, maneuver instruments, torpedo target, turret
  lead, health; Chrome: keybind hints, verb cues, edge indicators, target
  candidates, component lock, objectives); ephemeral world-space holos
  (ribbon segments, flip gate, orbit ring, radius spoke) tagged Instrument
  at their spawn sites; status bar tagged Chrome in nova_core.
- nova_menu absorbed: entering the menu drives the level to None, leaving
  restores All (replacing the bespoke status-bar hide; a mid-game level is
  intentionally reset by a menu round trip).
- Keybind hint cluster gained a "[`] HUD" discoverability row - driven,
  not static, because the cluster's documented invariant is "no rig, no
  keys, no hints" (the ledger's 20260708-165705 lesson; the static first
  version broke the existing invariant test exactly as the ledger
  predicted).

Verification:
- 3 new unit tests (cycle with per-press delivery guards; tier
  hide/restore across all levels; indicator-node overwrite via ancestor
  tier including the widget-redrives case) - plus the pre-existing 383
  nova_gameplay tests and 7 nova_menu tests all green.
- Throwaway e2e harness (not committed): real app, click New Game, press
  Backquote three times, asserting cluster+velocity Visibility at every
  level - full cycle PASS.
- 09_editor smoke green; Xvfb menu capture confirms the status bar stays
  hidden through the new mechanism.
- check/fmt clean; full local suite/clippy skipped per repo policy (CI).

Reflection: the two ledger lessons that applied here were both caught
before review this time - the producer/consumer schedule question shaped
the PostUpdate placement up front, and the keybind-hints invariant was
caught by the module's own regression test within minutes of the naive
static row. Reading LESSONS.md before the cycle demonstrably paid.
