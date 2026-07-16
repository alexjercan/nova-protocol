# Gauntlet Run 2.0: make it a real gauntlet - more gates, obstacles and hazards (parkour-map feel)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.7.0,scenario,content,modding


## Goal

Gauntlet Run (webmods/gauntlet) is the portal's first mod but a thin one:
four beacon gates in a line (START, GATE 1/2, FINISH) over a base-game
skybox. Make it a REAL gauntlet in the Minecraft-parkour-map sense: a course
that is fun to run and fail - many more gates forming an actual route,
obstacles to thread (dense asteroid corridors, tight turns, gravity wells to
sling or avoid), hazards that punish sloppy flying, and pacing (checkpoint
feel, escalating difficulty, a finish worth reaching). Keep it pure flying
skill - no combat required - so it stays the game's flight-feel showcase.

Ships as a version bump of the gauntlet portal mod (1.x), re-published via
nova_portal_gen; enabled-state-preserving update is part of the dogfood.

## Steps

- [ ] Read the focused spike (tasks/20260716-174631/SPIKE.md) and the current
      webmods/gauntlet/gauntlet.content.ron end to end; keep the file's INVARIANT
      header (gate areas must not overlap) as the top constraint.
- [ ] Design the route on paper first: START + 6 ordered gates + FINISH across
      three escalating acts (warmup / slalom / hazard), with real direction
      changes and verticality. Record each object's position, gate `area_radius`
      (aim ~18-22u, tighter than today), and pairwise gate-area separation so no
      two areas overlap. This layout is content - keep it deterministic.
- [ ] Rewrite gauntlet.content.ron OnStart to spawn the racer + all gates + the
      `gate` counter (1..=7) + act-1 objective/marker chrome. Keep the racer
      loadout (reinforced hull gives crash tolerance); drop the turret if it
      reads as combat kit (pure-flight course, no target needed).
- [ ] Add the obstacle field: one ScatterObjects asteroid corridor (Box region,
      fixed seed, invulnerable rocks so they can't be shot away) threaded between
      two act-2 gates, plus a couple of big solo invulnerable rocks to slalom.
      Compute the flyable gap against the 3.5-6.0x geometric factor (import the
      ASTEROID_GEOMETRIC_FACTOR_MIN/MAX constants in the rig) - do not eyeball.
- [ ] Add the hazard section: one big asteroid with `surface_gravity` (a gravity
      well) placed OFF the immediate gate approach line in act 3, so it pulls the
      ship/autopilot to sling-or-avoid without soft-locking a gate thread.
- [ ] Extend the ordered-gate OnEnter chain to all gates: each advances `gate`,
      detaches/attaches the objective marker, sets the next objective text, and
      swaps the skybox at act boundaries (cubemap -> cubemap_alt -> cubemap_alt2).
- [ ] Add the outcome frames: Outcome(Victory, "...") on the FINISH OnEnter, and
      Outcome(Defeat, "...") on an OnDestroyed handler for the player ship
      (crashed out) - gate the Defeat handler so it cannot fire after Victory.
- [ ] Bump webmods/gauntlet/gauntlet.bundle.ron meta.version 1.0.0 -> 1.1.0 and
      update its description; refresh webmods/gauntlet/README.md.
- [ ] Regenerate the portal preview: run scripts/preview-web.sh's nova_portal_gen
      invocation to write web/dist/mods; confirm the new 1.1.0 tree + catalog.json
      and that no stale 1.0.0 dir is left behind (clear the out dir if the
      generator does not prune).
- [ ] Write a production-faithful behavior rig (new
      crates/nova_assets/tests/gauntlet_course.rs, modeled on
      broadside_assault.rs): include_str the committed content.ron, register
      handlers loader-faithfully, and assert - gates advance ONLY in order, all
      gate areas are pairwise non-overlapping, each scatter corridor leaves a
      flyable gap across the whole geometric-factor range, FINISH raises Victory,
      player OnDestroyed raises Defeat and cannot flip a won course. Run it with
      a unifying sibling crate (serde feature-unification quirk).
- [ ] Verify: cargo fmt --check; run gauntlet_course + webmods_validation (with a
      unifying sibling) and the nova_portal_gen generate tests. Report skips
      honestly per the local-tests policy.
- [ ] Write tasks/20260716-124722/NOTES.md: the route/act design, the hazard
      choices, the geometric-factor gap math, and the deferred feel/balance
      playtest verdict (crash damage, well strength, hull margin) as a hands-on
      follow-up per the spike.

## Notes

- Spike: tasks/20260716-122954/SPIKE.md (v0.7.0 release scope, amended)
- Focused spike: tasks/20260716-174631/SPIKE.md - RECOMMENDED direction A
  (pure-data content.ron rewrite + v1.1.0 republish, no engine changes). Read
  it first; it names the exact primitives and the five authoring hazards below.
- Plan: docs/plans/20260716-v0.7.0-plan.md, strand 1
- Primitives 2.0 is built from (all exist today): ScatterObjects (Box/Ring,
  deterministic seed, asteroid_radius range) for dense corridors; Asteroid with
  `invulnerable: true` (solid wall that can't be shot away), `surface_gravity`
  (gravity well to sling/avoid), collision = bcs impact damage (the punish);
  Beacon `area_radius` gates (ordered by the proven `gate` counter);
  Outcome(Victory) at FINISH + Outcome(Defeat) on player-ship OnDestroyed;
  per-act SetSkybox across the three cubemaps (cubemap/_alt/_alt2).
- AUTHORING HAZARDS the plan must respect (spike section "Recommendation"):
  (1) keep all gate areas pairwise NON-overlapping or the race soft-locks -
  assert it in the rig; (2) asteroid geometric factor is 3.5-6.0x nominal, so
  MEASURE each corridor's flyable gap against that whole range, don't eyeball;
  (3) keep gravity-well SOI (~8x geometric radius) off the gate approach lines;
  (4) crash-damage/hull-margin is a PLAYTEST verdict, ship structure then tune;
  (5) republish cleanly (watch for a stale 1.0.0 dir in web/dist/mods) and keep
  webmods_validation green.
- Test rig: model on crates/nova_assets/tests/broadside_assault.rs
  (production-faithful behavior tests); webmods_validation.rs is the load gate.
- Republish: bump webmods/gauntlet/gauntlet.bundle.ron 1.0.0 -> 1.1.0, regen via
  nova_portal_gen (scripts/preview-web.sh). Enabled state is id-keyed, so the
  update preserves the toggle - that's the enabled-state-preserving dogfood.
- Course variety wants the asset variety pack (20260716-123544): themed skybox
  and asteroid texture variants would make the course read as a place; that art
  upgrade is a later, non-blocking pass. Build 2.0 on existing base textures now.
- Timing/score is DEFERRED to the follow-up 20260716-174729 (visible timer +
  clean-run bonus) - it needs a HUD timer readout the vocabulary lacks today.
