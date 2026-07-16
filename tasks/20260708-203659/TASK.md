# Playable capital-combat vertical-slice scenario

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.7.0, example, scenario

Spike: tasks/20260708-203517/SPIKE.md

North-star demo for the "shippable game" identity: one real capital-combat fight
that dogfoods every juice + combat system end to end, rather than the isolated
test-range examples v0.4.0 built. Your ship vs an enemy that launches torpedoes
you must screen with the PDC, with sections coming apart under fire.

Goal: a single playable scenario that exercises the whole loop - handling, PDC
screening, torpedo offense/defense, section destruction, audio, hit FX, and HUD -
under a win/lose frame.

Depends on: the v0.5.0 feel systems (handling, audio 162011, hit FX 162013,
smarter AI 162012) - all shipped. Build it on the RON scenario format
(20260525-133029) so the flagship scenario also dogfoods the authoring format.

This task now OWNS the win/lose frame. The legacy objectives tasks (133026/133027)
were closed 20260714: the objective foundation and HUD conveyance shipped in
v0.5.0, and an explicit win/lose state was never built - it belongs here, on top
of the RON format, as part of the vertical slice rather than as a standalone
legacy task.


## v0.7.0 (20260716, spike tasks/20260716-122954/SPIKE.md)

Pulled into v0.7.0 as the flagship content task (p85): the RON scenario format
it was gated on shipped in v0.6.0, so it is unblocked. Still owns the explicit
WIN/LOSE frame (the game has none). Include torpedo detonation/impact visual
polish (the projectile.rs explosion-visuals TODO) as slice juice. Plan:
docs/plans/20260716-v0.7.0-plan.md, strand 1.

## Placement (added 20260716, user direction)

Ships in the BASE game, surfaced twice: wired into the New Game progression
as the next scenario after the current chain (the base storyline continues
into it via NextScenario), AND listed as a standalone entry in the Scenarios
picker tab so it can be played directly. The alt-storyline campaign mod
(20260716-123535) is the portal counterpart; this one is core content.

## Steps (planned 20260716, /flow)

- [x] Builder module crates/nova_assets/src/scenario/broadside.rs (working
      id "broadside"; pick final name/copy during implementation - story
      continuity: the scavengers from shakedown_run return in force): staging
      with the alt skybox (cubemap_alt), a seeded asteroid-cover
      ScatterObjects field, player ship with full loadout (turret + torpedo
      bay), enemy capital ship - controller + reinforced hull sections +
      2x better_turret + torpedo_section + thruster - with AIControllerConfig
      (patrol + leash so it commits when the player closes).
- [x] Verify-first: confirm the AI engage range and torpedo launch-envelope
      constants in crates/nova_gameplay/src/input/ai.rs (survey claims
      engage ~600u; launch range band 1x-4x blast_radius with forward
      alignment > 0.5), then size arena distances so the enemy actually
      launches torpedoes at the player through the approach.
- [x] Beats/events in the builder: contact objective (approach + lock),
      screen-the-torpedoes beat (HintEmphasis on combat stance + radar lock;
      the PDC can shoot down committed torpedoes - verified, 1 HP sections,
      silent fizzle), kill objective; win: OnDestroyed(enemy) ->
      ObjectiveComplete + Outcome(Victory) with NO queued next (Enter returns
      to menu per 20260716-125856); lose: OnDestroyed(player) ->
      Outcome(Defeat) + NextScenario("broadside", linger: true) retry.
- [x] Register in build_scenarios() (crates/nova_assets/src/lib.rs:73),
      non-hidden with a real description (placeholder thumbnail from existing
      assets; real art comes with 20260715-220011); regenerate
      assets/base/scenarios/broadside.content.ron via the content_ron_parity
      test.
- [x] New Game chain: shakedown_run's final beat (the "done" objective after
      the pirate dies, crates/nova_assets/src/scenario/shakedown.rs) gains
      NextScenario("broadside", linger: true) + objective copy prompting
      [Enter] when ready; regenerate shakedown_run.content.ron.
- [x] Self-driving example examples/19_broadside.rs on the 01-12 curriculum
      pattern (BCS_AUTOPILOT + assertions + completion backstop): script the
      win path (close, engage, enemy destroyed -> assert Victory overlay up)
      and a defeat-path assertion (scripted player destruction -> Defeat
      overlay + lingering restart queued); add to the CI smoke list
      (tests/examples_smoke.rs).
- [x] Balance playtest under Xvfb: tune enemy torpedo cadence/magazine,
      turret damage, hull counts for a fair first-try-losable fight; record
      the numbers and reasoning in tasks/20260708-203659/NOTES.md; screenshot
      eyeball of the arena (render-output-eyeball).
- [x] Drive-by: delete the stale TODO at
      crates/nova_gameplay/src/sections/torpedo_section/projectile.rs:56 -
      detonation visuals shipped in v0.6.0 (particle burst + blast sphere,
      render.rs:139-344); the "detonation FX juice" scope collapses to this
      unless the playtest says otherwise - if it does, file a new task with
      the observation, do not widen this one.
- [x] Docs per keeping-docs-in-sync: player wiki scenarios page, CHANGELOG
      [Unreleased] (Scenarios & Objectives), dev wiki only if the authoring
      surface changed.
- [x] Verify: cargo check/fmt, new example green in the smoke suite, both
      outcome paths eyeballed.

## Notes (from the 20260716 planning survey; file:line verified in-repo)

- AI already fires turrets AND launches torpedoes (crates/nova_gameplay/src/
  input/ai.rs - update_torpedo_section_input, per-bay cooldown ~4s default);
  AI turrets have a point-defense override vs inbound committed torpedoes.
  Retreat state is a stub inheriting Engage (task 20260709-225734 lands it
  later; the slice enemy fights to the death, which is correct here).
- Player-side screening: turrets target COMMITTED torpedoes
  (TorpedoTargetChosen) via the radar lock path; shot-down torpedoes despawn
  without detonating (torpedo_section/mod.rs:366-406).
- Player death today: sections explode, root despawns, OnDestroyed fires,
  nothing else - the Outcome frame (20260716-125856) supplies the missing
  presentation.
- Depends on: 20260716-125856 (outcome frame). Same-cycle sibling; land it
  first.
- Spike: tasks/20260716-122954/SPIKE.md; plan docs/plans/20260716-v0.7.0-plan.md.

## Scope direction (user, 2026-07-16, mid-flow)

The scenario is NOT just the one fight - go ham on the story. The enemy can
carry multiple weapons; there can be multiple enemies, neutrals, and other
actors around the arc. Update the Steps' single-enemy framing accordingly
when this task is picked up: keep the capital duel as the climax the systems
were built for (PDC screening, torpedo offense/defense, section
destruction), but stage it inside a real story beat structure (approach,
escalation, twist) rather than a bare arena. The Outcome frame + buttons
shipped in 20260716-125856; this task consumes it.

- Follow-up from the outcome-frame review (R1.8, 2026-07-16): the
  asteroid_field/asteroid_next chain still has silent lingering
  NextScenario beats (incl. a death restart) with no Outcome - retrofit
  them when this task lands the outcome vocabulary across base content.

## As-built deltas (2026-07-16, /work)

- The scenario grew from "one fight" into a three-act story per the user's
  mid-flow scope direction: NEUTRAL hauler (new authorable
  `SpaceshipConfig.allegiance` surface), two-corvette ambush, gunship
  climax. Design record + measured numbers: NOTES.md.
- Survey correction: torpedo cadence is 10s (AI_TORPEDO_COOLDOWN_SECS), not
  the planned step's ~4s claim; distances sized against the source.
- Found + fixed by the example: the loader's EAGER SkyboxConfig insert
  panics on any non-preloaded cubemap (bcs unwrap hazard) - broadside's alt
  sky and every future mod sky hit it. The loader now defers through
  PendingSkyboxSwap; boundary pin scenario_load_defers_the_skybox_install.
- Example 19 walks BOTH outcome paths in one run (defeat -> Retry reload ->
  victory) and self-ends; the smoke suite gained a documented second
  completion sentinel for self-ending examples.
- Asteroid-field R1.8 retrofits landed here (Defeat on death, Victory on
  zone-clear - the Victory half was lost in an edit retry and re-landed by
  this branch's review R1.1).
- Balance step: distances/staging authored against measured AI constants and
  exercised by the harness; FEEL tuning (fair, first-try-losable) is
  explicitly deferred to user playtest per the v0.7.0 plan's bug policy -
  a harness cannot judge fair.
- Victory eyeball: target/shots capture verified (gold banner, message,
  Main Menu, hint); Defeat variant eyeballed in cycle 1's probe. Real
  interactive balance/look pass remains the user's.
