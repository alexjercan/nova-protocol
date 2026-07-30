# Drawer shell + interaction model + objectives section (Tab, PauseStates::Drawer, slide anim, tab-handle anchor)

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.9.0,spike,feature,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Goal

The Tab ship-computer drawer's SHELL and interaction model, plus its first
section (expanded objectives). This task GATES the drawer's other sections and
211520's diegetic tuck-target. Design fixed by Spike:
tasks/20260721-211512/SPIKE.md - implement its recommendation, do not
re-litigate the architecture.

Scope (direction-level; /plan breaks into steps at pickup):

- Tab keybind that opens/closes a right-side drawer, hard-coded KeyCode::Tab in
  the spirit of nova_menu toggle_pause (runs in GameStates::Playing regardless
  of pause substate so it can also CLOSE while frozen); NOT in the Unpaused-gated
  flight input rig. O stays ORBIT; Tab avoids the collision.
- Pause + cursor via a new PauseStates::Drawer variant (option A2 in the spike):
  the variant carries overlay identity; generalize pause_clocks/release_cursor
  (and exit partners) to fire on any non-Unpaused state; flight/section gating
  (already in_state(Unpaused)) is unchanged. ESC from Drawer closes to Unpaused.
- Slide-in animation from the right edge via bevy_common_systems TweenPlugin
  (already wired for comms), with a backdrop fade; a collapsed tab HANDLE on the
  right edge.
- Expose the tab handle's screen anchor (component/resource holding its screen
  rect) as the tween TARGET for 211520's diegetic objective hand-off.
- A section framework the later sections (comms log, map, ship) slot into.
- First section: EXPANDED objectives, rendering bevy_common_systems
  GameObjectives (data already exists).

## Steps

- [x] **Audit the new state route (do FIRST, it sizes the task).** A new
      `PauseStates::Drawer` is a new entry into "frozen". Grep the workspace for
      every gate: `grep -rn "PauseStates" crates --include=*.rs`. Two suppression
      mechanisms exist - (1) the `in_state(PauseStates::Unpaused)` set-gates
      (`crates/nova_gameplay/src/plugin.rs:166`), already correct since `Drawer`
      is not `Unpaused`; (2) **19 observer self-guards** that check
      `== PauseStates::Paused` directly (observers bypass set-gating -
      `set-gates-miss-observers`; 10 in `crates/nova_gameplay/src/input/player.rs`
      from :902, the rest in `input/targeting.rs`, `audio.rs`,
      `camera_controller.rs`, etc). Write the list of all 19 sites into NOTES,
      classifying each as "means while-frozen -> widen to `!= Unpaused`" vs
      "intentionally ungated (e.g. releases clear held keys during pause -
      player.rs:900) -> leave". Also confirm `setup_pause_ui` /
      `DespawnOnExit(PauseStates::Paused)` are `Paused`-only (pause menu must NOT
      show for the drawer) and note the outcome setter `sync_outcome_pause`
      (`nova_menu/src/lib.rs:989`) is unaffected (sim frozen in Drawer -> no new
      outcome fires).
- [x] Add the `Drawer` variant to `PauseStates` in
      `crates/nova_gameplay/src/lib.rs:124` with a doc comment (Tab drawer
      overlay; frozen like `Paused`; only entered from `Unpaused`). Add the
      `Drawer => PauseStates::Unpaused` arm to the exhaustive `match` in
      `toggle_pause` (`crates/nova_menu/src/lib.rs:317`) so ESC closes the
      drawer.
- [x] Widen the 19 `== PauseStates::Paused` guards flagged in the audit to
      `!= PauseStates::Unpaused` (introduce a small `is_frozen(&State<PauseStates>)`
      helper next to the enum and use it, so the meaning is named once and future
      variants are covered). Leave the intentionally-ungated release paths as-is.
- [x] **Write the interaction-model tests FIRST (harness/App altitude) and watch
      them fail** (no Tab system yet): in a headless `App` rig with `GameStates` +
      `PauseStates` and a `PrimaryWindow`+`CursorOptions` entity (mirror the
      cursor-rig pattern in `nova_editor`'s `scenario_grab_hides_and_locks...`
      test): `tab_toggles_drawer_state` (Playing/Unpaused + Tab -> `Drawer`; Tab
      again -> `Unpaused`), `esc_closes_drawer_to_unpaused` (in `Drawer`, ESC ->
      `Unpaused`, NOT `Paused`), `entering_drawer_freezes_clocks_and_frees_cursor`
      (on enter `Drawer`: `Time<Virtual>` paused + cursor visible/ungrabbed; and
      `would-it-fail-without-it` - delete the OnEnter(Drawer) hook and it goes
      red), and `flight_input_inert_while_drawer_open` (a `FlightBurnInput` in
      `Drawer` does not change `FlightIntent` - pins the guard widen).
- [x] Wire the Drawer freeze/cursor in nova_menu's build
      (`crates/nova_menu/src/lib.rs:221`): register
      `OnEnter(PauseStates::Drawer) -> (pause_clocks, release_cursor)` and
      `OnExit(PauseStates::Drawer) -> (unpause_clocks, restore_cursor)`, mirroring
      the `Paused` wiring but WITHOUT `setup_pause_ui`.
- [x] Add the drawer module `crates/nova_gameplay/src/hud/drawer/` (or
      `hud/drawer.rs`): a `toggle_drawer` system keyed on `KeyCode::Tab`,
      `run_if(in_state(GameStates::Playing))` (NOT in the `Unpaused`-gated flight
      rig, so it can also CLOSE while frozen), guarding on a live outcome like
      `toggle_pause` does. Matches `Unpaused <-> Drawer`. Register a
      `NovaDrawerPlugin` in `NovaHudPlugin` (`hud/mod.rs:155`). Make the state
      tests pass.
- [x] Drawer surface: a right-side panel spawned `OnEnter(Drawer)` /
      `DespawnOnExit(PauseStates::Drawer)`, sliding in from the right edge via the
      bcs `TweenPlugin` already wired at `hud/mod.rs:198` (reference
      `hud/comms_panel.rs` `apply_comms_fade` for the tween-progress pattern;
      `verify-bevy-api-at-callsite`), with a dim backdrop. A collapsed tab HANDLE
      on the right edge, visible while the drawer is closed.
- [x] Expose the tab-handle screen anchor as a resource/component
      (`DrawerTabAnchor`, holding the handle's screen rect), updated each frame -
      this is task 20260721-211520's tween target. Test
      `drawer_exposes_tab_handle_anchor` asserts it is present and tracks the
      handle node.
- [x] Section framework + first section: a sections container in the drawer whose
      first section renders the expanded `GameObjectives`
      (`bevy_common_systems`, already synced; compact panel at `hud/mod.rs:273`).
      Decide + document (NOTES, `does-the-old-element-survive`) that the drawer
      overlays independently of the grave/tilde `HudVisibility` cycle and the
      top-right compact objectives panel - both stay; the drawer is a separate
      axis. Test `drawer_objectives_section_lists_objectives`.
- [x] Verify: `cargo check --all-targets` (the new enum variant breaks any
      exhaustive match / example - `check-all-targets-for-struct-field`), `cargo
      fmt`, the new tests, and `cargo doc -p nova_gameplay -p nova_menu --no-deps`
      (rustdoc intra-links). Run a probe of a gameplay example (drawer-closed path
      must be unchanged) per the probe skill.
- [x] Docs sweep (`keep-docs-in-sync-with-code`, x8): add the Tab drawer to
      `web/src/wiki/keybinds.md` and `web/src/wiki/hud.md`, and a CHANGELOG
      `[Unreleased]` line under **Interface & HUD**. `grep -rn` the whole doc tree
      for the keybind table and objectives/comms mentions the drawer changes.

## Definition of Done

- Tab opens and closes the drawer from flight, and ESC closes it to `Unpaused`
  (not the pause menu) (test: `tab_toggles_drawer_state`,
  `esc_closes_drawer_to_unpaused`).
- Entering `Drawer` freezes the sim clocks and frees+shows the cursor; leaving
  restores both (test: `entering_drawer_freezes_clocks_and_frees_cursor`, which
  goes red if the `OnEnter(Drawer)` hook is removed).
- Flight input is inert while the drawer is open - the observer guards suppress
  in `Drawer`, not just `Paused` (test: `flight_input_inert_while_drawer_open`);
  no `== PauseStates::Paused` "while-frozen" guard remains
  (cmd: `grep -rn "== crate::PauseStates::Paused\|== PauseStates::Paused" crates --include=*.rs`
  returns only the intentionally-ungated/​non-freeze sites listed in NOTES).
- The drawer exposes its tab-handle screen anchor for task 20260721-211520
  (test: `drawer_exposes_tab_handle_anchor`).
- The expanded objectives section renders the current `GameObjectives` inside the
  drawer (test: `drawer_objectives_section_lists_objectives`).
- The pause menu does NOT spawn when the drawer opens (test asserts no pause-panel
  entity in `Drawer`; part of the freeze/UI test).
- The A2 pause-axis + Tab-keybind decision is recorded
  (cmd: `test -f tasks/20260724-102304/DECISION.md`).
- The Tab drawer is documented (cmd: `grep -ni drawer CHANGELOG.md web/src/wiki/keybinds.md`).
- manual: in a real run the owner opens the drawer with Tab - it slides in from
  the right, the game pauses, the cursor appears, objectives show expanded, and
  the tab handle is visible when closed; Tab and ESC both close it; the slide
  animation reads well.
- Overall: `cargo check --all-targets` clean, `cargo fmt` clean, new tests green,
  and a probe run of a gameplay example returns OK/WARN with the drawer-closed
  path unchanged.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). This task carries the
  load-bearing DECISION.md (tasks/20260724-102304/DECISION.md) for the A2
  pause-axis + Tab-keybind choice, citing the spike as context.
- Builds on the state-driven cursor from 20260721-211500 (CLOSED).
- Gates 20260721-211520 (needs the tab-handle anchor) and the comms-log section
  (20260724-102309).
- Grounded facts (verified 2026-07-24): `PauseStates` enum
  `crates/nova_gameplay/src/lib.rs:124`; freeze/cursor hooks + `toggle_pause`
  match `crates/nova_menu/src/lib.rs:221,296,317,333,351`; flight/section
  set-gate `crates/nova_gameplay/src/plugin.rs:166`; 19 `== Paused` observer
  guards (10 in `input/player.rs` from :902); `NovaHudPlugin` `hud/mod.rs:155`,
  bcs `TweenPlugin` `hud/mod.rs:198`, objectives panel `hud/mod.rs:273`; comms
  tween reference `hud/comms_panel.rs`. Bevy 0.19.
- Touches TWO crates: nova_gameplay (enum variant, guard widen, drawer module,
  Tab toggle, UI, objectives section) and nova_menu (Drawer freeze/cursor hooks,
  `toggle_pause` match arm).

## Close-out (2026-07-24)

Shipped the Tab drawer shell + interaction model + objectives section. See
DECISION.md (A2 pause-axis) and NOTES.md (guard audit, animation-clock and
does-the-old-element-survive decisions).

What changed:
- `PauseStates` gained a third variant `Drawer` + an `is_frozen()` helper
  (`crates/nova_gameplay/src/lib.rs`). `nova_menu` wires
  `OnEnter/OnExit(Drawer)` to the SAME `pause_clocks`/`release_cursor` (and exit
  partners) as `Paused`, minus `setup_pause_ui`; `toggle_pause` grew a
  `Drawer => Unpaused` arm so ESC closes the drawer.
- Widened 18 of 19 observer `== Paused` self-guards to `is_frozen()` (the 19th,
  `sync_outcome_pause`, is intentionally precise - see NOTES). Behavior-
  preserving for the pre-existing states, so no existing test changed.
- New `hud/drawer.rs` (`NovaDrawerPlugin`): `KeyCode::Tab` toggle
  (`Playing`-gated, not flight-rig gated), a right-side sliding panel + dim
  backdrop + always-on tab handle, the `DrawerTabAnchor` resource (task
  20260721-211520's tween target), and the objectives section rendering
  `GameObjectives`.

Difficulties / decisions:
- The bcs `Tween` advances on `Res<Time>` (= `Time<Virtual>`), which the drawer
  PAUSES - so a bcs tween would freeze mid-slide. Verified against the bcs source
  and drove the slide with `Time<Real>` instead (deviates from the plan's "via
  TweenPlugin" note; recorded in NOTES). This is exactly the
  `verify-engine-guarantees-in-source` check paying off.
- The audit (`audit-state-gates-on-new-entry-path`) was the real bulk: observers
  bypass set-gating (`set-gates-miss-observers`), so the 19 hand guards - not the
  `in_state(Unpaused)` set-gate - were where the new state could leak.
- Two test-rig bugs (not code): manual `ButtonInput` needs an explicit `clear()`
  between presses (mirrored `press_escape`), and the flyable-ship test rig does
  not attach `FlightIntent` by default.

Verification: 7 new tests green (5 nova_gameplay drawer/input + 2 nova_menu
freeze/ESC); `cargo check --workspace --all-targets` clean; `cargo fmt` clean;
`cargo doc -p nova_gameplay -p nova_menu` warning-free; `cargo run -p nova_probe
-- run gameplay` -> aggregate OK (scenario/playable/broadside/lifeline all OK,
invariants held, drawer-closed path unchanged). The `manual:` slide-feel item is
batched for owner acceptance at flow Finish.

Self-reflection: the plan's up-front audit step (sizing the 19 guards) made the
risky part mechanical - worth the planning cost. Next time, verify a dependency's
clock/ordering assumption (the bcs Tween clock) at PLAN time when the design
leans on it, rather than discovering it mid-implementation; it was cheap here but
could have forced a redesign.
