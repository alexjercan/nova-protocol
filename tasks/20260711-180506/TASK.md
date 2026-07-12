# Starter New Game scenario: fun but gentle

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.5.0,scenario,content,spike

## Goal

The scenario New Game actually drops you into: "Shakedown Run" (id
`shakedown_run`), a five-beat first-flight tutorial per the spike's beat
sheet - burn to beacon, freelook find, salvage sweep, GOTO/ORBIT hands-off,
then a pirate that spawned in the debris cluster as the finale. Legs a few
hundred meters, ships minimal (one turret each), conveyance is layer 0
(imperative text with [KEY] names, emissive blinking props, short
distances). Ends with New Game loading it instead of asteroid_field.

## Steps

- [x] Verify TurretSectionConfig's damage/fire-rate fields, then register
      scavenger-grade sections in `crates/nova_assets/src/sections.rs`.
      AS EXECUTED: bullet damage is kinetic (bcs integrity impulse/energy
      modifiers), so `light_turret_section` tunes fire_rate 100->25,
      muzzle_speed 100->60, projectile_mass 0.1->0.05 (~1/5 per-hit
      energy); `light_hull_section` is 60 health vs reinforced 200.
- [x] New module `crates/nova_assets/src/scenario/shakedown.rs` (child of
      scenario.rs): `shakedown_run(...) -> ScenarioConfig` with the whole
      layout as named constants. AS EXECUTED, geometry derived from the
      RUNTIME numbers (authored-vs-derived lesson): geometric planetoid
      radius ~4.0-4.55x nominal, SOI = 8x geometric (640-730u), ORBIT ring
      = 1.5 * (body_radius + 1) = ~122-138u. Player at origin; beacon 1 at
      350u ahead; beacon 2 ~120 degrees off boresight; debris cluster (9
      fixed-offset rocks + 3 crates) past beacon 2; planetoid ~1050u from
      spawn (SOI edge crossed on the beat-4 leg); beacon 3 at ~220u from
      the planetoid center; orbit gate sphere at 160u. Rocks are fixed
      offsets, not rng - determinism keeps the config tests honest.
- [x] Player ship: LMB turret mapping, controller + reinforced hull +
      thruster + one better_turret - no hull_back, no torpedo bay.
      Pirate: AI patrol over the debris cluster, controller + light hull +
      thruster + one light_turret; spawned by the salvage-complete
      handler, not OnStart.
- [x] Beat chain events. AS EXECUTED, two structural choices beyond the
      plan: (1) a single `beat` counter variable gates every handler
      (finished beats cannot re-fire; asteroid_field's per-flag idiom
      would have needed 5 flags); (2) count milestones (tally text, beat
      advance) run on OnUpdate handlers keyed on the counter value,
      NOT on the pickup event - handler execution order within one event
      is query-iteration order and must not be load-bearing. Lazy spawns:
      beacon 2 appears with beat 2, beacon 3 + pirate with beat 4, so a
      new HUD chip always means "this is next". Early-pirate-kill branch:
      killing it during beat 4 sets a flag and the orbit gate routes
      straight to done. Player death -> NextScenario(shakedown_run,
      linger).
- [x] ADDITIONAL (found during work): `EventConfig::OnUpdate` was DEAD
      CONFIG - the variant and docs existed but nothing ever fired
      OnUpdateEvent, so the milestone handlers could never run. Added the
      `fire_on_update` pulse to ScenarioLoaderPlugin (Update, gated on
      scenario_is_live), with a loader test proving it pulses only while
      a scenario is live.
- [x] Register `shakedown_run` in register_scenario and flip
      NEW_GAME_SCENARIO_ID to "shakedown_run" (nova_menu reads the const
      everywhere, including its tests, so the swap is one line).
- [x] Config-shape tests (7) + full-script walk (2): every referenced id
      is spawned (the script is strings; a typo fails silently); pirate
      spawns late, exactly once, patrolling the cluster; ships minimal
      and scavenger-grade where required; beat-4 geometry pinned against
      the WORST derived-radius seed (beacon 3 inside smallest SOI,
      outside the gate; widest ring inside the gate); death routes home
      with linger; every gameplay handler carries a variable guard. Then
      `the_five_beats_walk_end_to_end`: the real handlers registered
      exactly as the loader registers them, real spawn/despawn against a
      real World, all five beats driven by the same events production
      fires (plus the stray-re-entry non-refire case), and
      `early_pirate_kill_routes_the_orbit_gate_to_done` for the alternate
      ending. The physics half of OnEnter is owned by 20260712-093044's
      pipeline test; registration guarded by shakedown_run_is_registered.
- [x] Update docs: CHANGELOG (scenario + primitives entries),
      docs/scenario-system.md (shakedown_run as the beat-chain reference,
      OnUpdate now actually fired).
- [ ] Visual playtest (boot -> New Game -> walk the beats by hand,
      screenshots): NOT DONE - needs a human at the controls; the
      headless walk covers the script, not the feel. First playtest owns:
      beacon visual size/blink readability, crate pickup radius feel,
      pirate difficulty, orbit-gate moment, objective text length.

## Notes

- Spike (design, beat sheet): docs/spikes/20260712-092926-starter-scenario.md
- Spike (parent direction): docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (CLOSED), 20260712-093044 (CLOSED)
- Conveyance is layer 0/1 by design; task 20260712-093831 upgrades
  visuals later without touching the beat chain (targets are scenario
  entity ids).
- Objective text uses the exact key labels the hint cluster shows for the
  flight verbs; Alt/RMB/LMB are named in text only (the cluster covers
  flight verbs, not camera/combat modifiers).
- The orbit gate is an area approximation of "in orbit"; if playtest says
  it fires too early/late, tune ORBIT_GATE_RADIUS (200u vs ring 106-182u
  across the pinned factor range) before reaching for autopilot-state
  events - and keep it above the widest ring
  (1.5 * (20 * ASTEROID_GEOMETRIC_FACTOR_MAX + 1)) or beat 4 soft-locks
  on high seeds (review R1.2).

## Close record

What changed: scenario/shakedown.rs (the whole scenario: layout
constants, expression/action shorthands, 14 handlers), two scavenger
sections in sections.rs, registration + NEW_GAME_SCENARIO_ID swap, the
fire_on_update pulse in nova_scenario's loader (OnUpdate handlers were
dead config), CHANGELOG + scenario-system.md, 10 new tests.

Alternatives considered: per-beat boolean flags (asteroid_field idiom) vs
the beat counter - counter chosen for single-guard gating; milestones on
the pickup event relying on handler order (asteroid_field does this for
its threshold check) vs OnUpdate value-gating - the latter chosen and the
missing pulse built, because handler order within an event is
query-iteration order and should not carry gameplay.

Difficulties: the dead OnUpdate discovery (grep for the event's fire site
before building on an advertised config variant - the docs said "every
frame", the code never fired it); f64 vs f32 in VariableLiteral::Number.

Self-reflection: the OnUpdate gap is the same shape as task 1's targeting
gate - an advertised capability whose consumer side was never wired; both
were caught by asking "who actually fires/admits this" before trusting
the config surface. The visual playtest step was planned as if the agent
could fly the game; plans should split "script walk (automatable)" from
"feel pass (human)" up front.

Review addendum (round 1 -> 2): the fresh-context review caught two
MAJORs the implementation missed - the OnUpdate pulse making the
objectives panel rebuild every frame (fixed with a write-on-diff sync,
mutation-tested regression), and the geometric-factor band being
folklore: a 256-seed sweep measured [3.70, 5.64] vs the assumed 4.55, so
the orbit ring could park OUTSIDE the old 160u gate (a live softlock).
Bounds are now exported consts pinned by the sweep; gate 200u, beacon 3
at ~260u. APPROVE in round 2 (tasks/20260711-180506/REVIEW.md).
