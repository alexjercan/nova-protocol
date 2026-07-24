# Diegetic objective presentation: big on the cockpit HUD, then tucks into the right tab

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.9.0,feature,hud,ui

## Goal

Owner direction (playtest, 2026-07-21): objectives should appear
DIEGETICALLY - imagine a HUD cockpit: the new objective appears on it "a
bit rotated and big", holds, then animates away INTO the right tab (the
future Tab drawer's handle), where it lives in the compact list. The
right-tab list gains more detail and expands via the drawer (own spike).

v0.9.0 candidate; the Tab spike (see Notes) owns the family's interaction
design - this task implements the objective-presentation piece once the
spike lands. /plan breaks it into steps at pickup.

## Steps

- [ ] Verify-first (grounding, mostly done at plan time): the animation vehicle
      is `UiTransform { translation: Val2, scale: Vec2, rotation: Rot2 }` (Bevy
      0.19 UI, used across the HUD - `beacon_chips.rs:99`, `objective_markers.rs`,
      `screen_indicator.rs`); `Rot2::degrees`, `Val2::px`. The tuck TARGET is
      `DrawerTabAnchor.rect` (pub, `crates/nova_gameplay/src/hud/drawer.rs`,
      updated each frame from the drawer tab handle - `Option<Rect>`, `None`
      before first layout). The new-objective detection point is
      `objective_feedback.rs` `objective_change_feedback`'s `added_objectives`
      (already diffs `GameObjectives` by id; write-on-diff, so `resource_changed`
      is a real change). This animation plays during normal flight (Unpaused), so
      the virtual clock / `Res<Time>` is correct here (unlike the drawer slide).
- [ ] Write the reveal test FIRST (harness altitude) and watch it fail: in a
      `MinimalPlugins` app with `GameObjectives`, the reveal plugin and a
      `DrawerTabAnchor` seeded with a known rect, post a new objective and assert
      a reveal card spawns (an `ObjectiveRevealMarker` + a `Text` equal to the
      objective message); step frames through appear + hold + tuck and assert the
      card's `UiTransform.translation` moves toward the anchor and the card
      despawns after the total duration
      (test: `objective_reveal_spawns_and_tucks_to_the_anchor`). Delete the
      animate system -> the card never moves/despawns (would-it-fail-without-it).
- [ ] New module `crates/nova_gameplay/src/hud/objective_reveal.rs`:
      `ObjectiveRevealMarker { elapsed, message }` + constants (appear ~0.3s, hold
      ~2.3s, tuck ~0.5s; big scale ~2.0 -> ~0.3; rotation ~-5deg; using bcs
      `EaseFunction` for the curve). `spawn_objective_reveal(commands, objective)`
      spawns a big rotated absolute-centered card. `animate_objective_reveals`
      advances `elapsed`, runs the appear -> hold -> tuck phase machine, writes
      `UiTransform` (scale/rotation/translation toward `DrawerTabAnchor.rect`
      centre, falling back to the top-right compact-panel position when the anchor
      is `None`) and fades alpha, and despawns the card when spent. Teardown:
      emptying `GameObjectives` despawns any in-flight reveal
      (`state-diff-aliases-reset`, mirror the ghost teardown). `ObjectiveRevealPlugin`.
- [ ] Route additions to the reveal in `objective_feedback.rs`
      (does-the-old-element-survive): the fresh-posting gold ghost line
      (`added_objectives -> OBJECTIVE_GOLD`, task 20260717-163033) is SUPERSEDED
      by the big reveal - replace that spawn with a `spawn_objective_reveal` call
      so a new objective does not double-animate. COMPLETIONS keep their green
      ghost line unchanged. Document the replacement in NOTES + close-out.
- [ ] Register `ObjectiveRevealPlugin` in `NovaHudPlugin` (`hud/mod.rs`).
- [ ] Verify: `cargo check --all-targets`, `cargo fmt`, the new tests,
      `cargo doc -p nova_gameplay --no-deps`. Probe a scenario that posts
      objectives (`gameplay/scenario` or `playable`) - invariants held, log clean,
      no regression to the objective path.
- [ ] Docs sweep: CHANGELOG `[Unreleased]` under **Interface & HUD**; update
      `web/src/wiki/hud.md` "Comms and objectives" (the fresh-objective gold flash
      is now the big diegetic reveal that tucks into the drawer tab handle).

## Definition of Done

- A new objective spawns a big, slightly-rotated reveal card showing its text
  (test: `objective_reveal_spawns_and_tucks_to_the_anchor`).
- The card holds, then animates toward the `DrawerTabAnchor` (the drawer tab
  handle) and despawns after the total duration; removing the animate system
  makes the test go red (test: `objective_reveal_spawns_and_tucks_to_the_anchor`).
- Additions no longer spawn the old gold ghost line - the reveal replaces it -
  while completions still spawn the green ghost
  (test: `additions_route_to_reveal_not_gold_ghost`).
- Emptying `GameObjectives` (scenario teardown) despawns any in-flight reveal
  (test: `scenario_teardown_clears_reveals`).
- manual: in a real run a new objective appears large and slightly rotated on the
  cockpit HUD, holds ~2-3s, then tucks into the right tab handle and vanishes -
  reads well and lands into the drawer's tab.
- Overall: `cargo check --all-targets` + `cargo fmt` clean, new tests green, and a
  probe of an objective-posting scenario returns OK/WARN.

## Notes

- Depends on: 20260721-211512 (Tab drawer spike - design, CLOSED) and
  20260724-102304 (drawer shell - LANDED c13143d4, provides `DrawerTabAnchor`).
- Owner decision (questionnaire, 2026-07-21): the BIG COCKPIT MOMENT - the new
  objective appears large and slightly rotated on the cockpit HUD (~2-3s), then
  animates into the right tab.
- PACING is upstream and OUT OF SCOPE here: authored pacing gaps (20260721-211506,
  CLOSED) control WHEN objectives post so a reveal never lands mid-fight; this task
  only animates the reveal when an objective posts.
- Animation vehicle decided at plan time: a hand-rolled appear/hold/tuck phase
  machine on the normal clock (consistent with `objective_feedback.rs`'s
  `fade_ghost_lines` `age` pattern), eased with bcs `EaseFunction`. Not a
  DECISION.md - it is a local mechanism choice, not an architectural fork.
- Grounded facts (verified 2026-07-24): `UiTransform`/`Val2`/`Rot2` in bevy_ui
  0.19; `DrawerTabAnchor` pub in `hud/drawer.rs`; added-objective detection in
  `hud/objective_feedback.rs:170-247`; gold-ghost-for-additions at
  `objective_feedback.rs:224-243`.
