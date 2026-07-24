# Flight objective HUD: minimalist top-right status-bar notification (remove compact panel + tab square; retune reveal; gamepad open)

- STATUS: CLOSED
- PRIORITY: 66
- TAGS: v0.9.0,feature,ui,hud

## Goal

Playtest rework (owner, 2026-07-24): the always-on flight objective surface is
wrong. Replace it with a MINIMALIST top-right status-bar notification.

Owner feedback driving this:
- The old compact objectives panel (top-right text list) is still there - REMOVE
  it (objectives now reveal diegetically + live in the drawer).
- Dislikes the "drawer square" (the tab handle) sitting on the right during play -
  REMOVE it, or at most a tiny "Tab" hint. Preferred: a minimalist top-right
  status-bar notification ("objectives" etc.) that is terse (little text) and
  hints "Tab" to open, with a gamepad alternative.
- The diegetic objective reveal is too big + too centered; wants it a bit SMALLER
  and the vanish animation to translate toward the RIGHT (into this notification).

Scope (direction-level; /plan breaks into steps at pickup):

- Remove the always-on compact objectives panel (`hud/mod.rs` spawn_objectives_panel
  / the bcs ObjectivesPanel treatment) from flight; objectives live in the drawer's
  right panel now (task 20260724-102304 shell) + the diegetic reveal (211520).
- Remove the drawer tab-handle square (`hud/drawer.rs` DrawerTabHandleMarker) from
  the flight view; add a minimalist top-right notification in the status-bar strip
  (near `hud/readout.rs`) - terse (current objective one-liner or a count/icon),
  hinting "Tab". Keep it small; it is a status hint, not a panel.
- Repoint `DrawerTabAnchor` to this notification's screen rect (it is 211520's
  diegetic tuck target - the reveal should now tuck INTO the notification).
- Retune the reveal (`hud/objective_reveal.rs`): smaller card; vanish translates
  toward the notification (right).
- Add a GAMEPAD button to open/close the drawer (Tab has no pad equivalent yet;
  pick a free pad button - the flight rig uses bevy_enhanced_input; check
  reference.rs for a free one). Show the pad hint alongside "Tab".

## Steps

- [x] Verify-first (recorded at plan time): (a) the bcs `ObjectivesPlugin`
      (`hud/mod.rs:197`) inits `GameObjectives` AND its `rebuild_lines` uses
      `Single<.., With<ObjectivesPanelMarker>>`, which SILENTLY SKIPS when no
      panel exists - so dropping the nova compact-panel spawn is safe and
      `GameObjectives` stays populated for the drawer/reveal. KEEP `ObjectivesPlugin`.
      (b) `GamepadButton::RightThumb` (right stick click) is the free pad button
      (DPadDown = scenario-advance, RightTrigger2 = editor/scenario, LeftThumb =
      editor, face/DPad/triggers/Start/Select all taken).
- [x] Remove the compact objectives panel from flight (`hud/mod.rs`): drop the
      `setup_hud_objectives`/`remove_hud_objectives` observers, `spawn_objectives_panel`,
      `style_objective_lines`, the `ObjectivesPluginSystems::Sync` styling wiring,
      and now-unused `OBJECTIVES_PANEL_WIDTH_PX`/`OBJECTIVES_FONT_PX`. KEEP
      `ObjectivesPlugin` (it owns `GameObjectives`). Update/remove the
      panel-styling test at `mod.rs:~1075` - it exercises a removed element
      (does-the-old-element-survive). Grep `ObjectivesPanelMarker` for any other
      consumer.
- [x] New minimalist flight objective HINT (new module `hud/objective_hint.rs`,
      `ObjectiveHintPlugin`): a small top-right widget (top ~16px, right ~8px -
      beside the top-center readout strip, NOT on it), `HudTier::Chrome`,
      spawned/despawned with the player ship (mirror the other HUD widgets).
      Content (OWNER CHOICE 2026-07-24 - "just a count/icon + TAB"): an objective
      glyph + the ACTIVE-objective COUNT (`GameObjectives.objectives.len()`) + a
      small "TAB" affordance (+ the right-stick pad glyph) - NO per-objective text
      (the reveal + drawer carry the detail). Hidden when the count is 0. Marker
      `ObjectiveHintMarker`.
- [x] Repoint the diegetic tuck anchor to the hint: the hint module writes
      `DrawerTabAnchor` (stays pub in `hud/drawer.rs`) from `ObjectiveHintMarker`'s
      screen rect; REMOVE `DrawerTabHandleMarker` + its spawn + the handle-based
      `update_tab_anchor` from `hud/drawer.rs`. Update drawer tests:
      `drawer_exposes_tab_handle_anchor` (anchor now sourced from the hint) and
      `drawer_renders_above_the_hud` (no handle spawned; assert only panel +
      backdrop z).
- [x] Retune the reveal (`hud/objective_reveal.rs`): smaller card (reduce
      `REVEAL_BIG_SCALE`, `REVEAL_WIDTH_PX`, `REVEAL_FONT_PX`) and a base position
      that reads less dead-center; it already tucks toward `DrawerTabAnchor` (now
      the top-right hint), so the vanish translates up-and-right into the hint.
      Update the reveal test's base-position math if the base frac/size changes.
- [x] Gamepad open (`hud/drawer.rs` `toggle_drawer`): also fire on
      `GamepadButton::RightThumb` (mirror `toggle_pause`'s `Option<Res<ButtonInput
      <GamepadButton>>>` handling). Test `pad_toggles_drawer_state`.
- [x] Verify: `cargo check --all-targets`, `cargo fmt`, the drawer/reveal/hint/hud
      tests, `cargo doc -p nova_gameplay --no-deps`. Probe `playable` (posts
      objectives) - the hint spawns, the reveal plays, no compact panel, and
      `GameObjectives` still drives them. Docs: `web/src/wiki/keybinds.md` (drawer
      = Tab + right-stick-click pad glyph), `web/src/wiki/hud.md` (objectives are
      now a terse top-right hint + the drawer, not the compact panel), CHANGELOG
      Interface & HUD.

## Definition of Done

- The always-on compact objectives panel no longer spawns in flight
  (test: after a player spawns, no `ObjectivesPanelMarker` exists;
  cmd: `grep -n spawn_objectives_panel crates/nova_gameplay/src/hud/mod.rs` is empty).
- A minimalist top-right objective hint spawns in flight showing the active
  objective COUNT + glyph + a Tab affordance (no per-objective text), and hides at
  count 0 (test: `objective_hint_shows_count_and_hides_when_empty`;
  manual: reads minimal, hints Tab + pad).
- The drawer tab-handle square no longer spawns, and the reveal's
  `DrawerTabAnchor` is sourced from the hint
  (test: `drawer_exposes_tab_handle_anchor` updated to the hint;
  cmd: `grep -rn DrawerTabHandleMarker crates` is empty).
- The reveal is smaller and tucks toward the top-right hint
  (test: reveal scale/size reduced and tucks to the anchor;
  manual: reads smaller and slides up-right).
- Gamepad right-stick-click opens/closes the drawer
  (test: `pad_toggles_drawer_state`; manual: the pad opens it).
- `GameObjectives` still drives the drawer + reveal after the panel removal
  (probe: `playable` OK, objectives path intact).
- Docs updated (cmd: `grep -ni drawer web/src/wiki/keybinds.md web/src/wiki/hud.md`);
  `cargo check --all-targets` + `cargo fmt` clean; new + touched tests green.

## Notes

- From the 2026-07-24 playtest of the drawer family (shell 102304 + reveal 211520
  + z-order 121541, all LANDED). Files: hud/mod.rs (compact objectives panel),
  hud/drawer.rs (tab handle + DrawerTabAnchor + toggle_drawer), hud/objective_reveal.rs
  (reveal), hud/readout.rs (top status strip for placement), nova_menu toggle_pause
  (gamepad pattern), input/reference.rs (rig bindings).
- does-the-old-element-survive: this REMOVES the compact objectives panel AND the
  tab-handle square - grep their markers/spawns and the tests that assert them.
- Grounded (2026-07-24): bcs `ObjectivesPlugin` inits `GameObjectives` and
  `rebuild_lines` (`Single<..ObjectivesPanelMarker>`) skips with no panel - safe to
  drop the panel spawn, keep the plugin. `RightThumb` is the free pad button.
- Gate decisions RESOLVED (owner, 2026-07-24): gamepad button = `RightThumb`
  (right stick click); hint content = objective glyph + COUNT + TAB, no per-objective
  text. No DECISION.md - relocations + value choices, not an architectural fork.

## Close-out (2026-07-24)

Reworked the flight objective surface per the playtest.

What changed:
- REMOVED the always-on compact objectives panel (`hud/mod.rs`:
  spawn_objectives_panel / style_objective_lines / setup_hud_objectives /
  remove_hud_objectives + consts + the panel-styling test). KEPT bcs
  `ObjectivesPlugin` (it owns `GameObjectives`; its `rebuild_lines` is a `Single`
  system that skips with no panel). The green completion ghosts stay (their
  column width is now a local const in `objective_feedback`).
- NEW `hud/objective_hint.rs` (`ObjectiveHintPlugin`): a minimalist top-right
  widget - objective glyph + active-objective COUNT + a "TAB" chip - spawned with
  the player, `HudSelfDrivenVisibility`, hidden at count 0. It also publishes
  `DrawerTabAnchor` from its own screen rect.
- REMOVED the drawer tab handle (`hud/drawer.rs`: DrawerTabHandleMarker + spawn +
  the handle-based update_tab_anchor + consts). `DrawerTabAnchor` (the reveal's
  tuck target) is now written by the hint. Moved the anchor test to
  objective_hint.
- Retuned the reveal (`hud/objective_reveal.rs`) smaller (scale 1.9->1.35, width
  360->260, font 22->18); it already tucks toward the anchor, now the top-right
  hint, so the vanish slides up-and-right.
- Gamepad: `toggle_drawer` also fires on `GamepadButton::RightThumb` (owner
  choice; the one free pad button), mirroring toggle_pause's optional-gamepad guard.

Difficulties:
- The `pad_toggles_drawer_state` test first failed asserting after a single
  `update()`: a `NextState` set during Update applies on the NEXT frame, and
  without a `clear()` the stale `just_pressed` edge re-toggles - same shape as
  `press_tab`. Fixed with a `press_pad` helper (press+update, release+clear+update).
- One dangling consumer of the removed `OBJECTIVES_PANEL_WIDTH_PX`
  (`objective_feedback`'s ghost column) - gave it a local `GHOST_COLUMN_WIDTH_PX`.

Verification: 9 touched-module tests green (2 hint + 5 drawer incl. pad + 2
reveal) plus objective_feedback; `cargo check --workspace --all-targets` + `cargo
fmt` clean; `cargo doc -p nova_gameplay` warning-free; `cargo run -p nova_probe --
run playable` -> OK (1382 frames, invariants held, log clean - objectives still
drive the hint/reveal/drawer after the panel removal). Docs: CHANGELOG, keybinds
(pad glyph), hud.md, and a scenario-authoring guide line swept off "objectives
panel". Manual items (hint reads minimal + hints Tab/pad; reveal smaller + slides
right; pad opens the drawer) batched for owner acceptance.

Self-reflection: the `Single`-skips-when-absent guarantee made the panel removal
safe and let me keep `GameObjectives` without any resource-init churn - verifying
that at plan time (reading bcs `rebuild_lines`) was what made this a clean rework
rather than a resource-lifetime scramble. The `NextState`-applies-next-frame +
clear pattern for input tests has now bitten twice (Tab, pad); it is captured in
the retro/ledger.
