# Minimal faction/relation model (hostile/neutral/own)

- STATUS: CLOSED
- PRIORITY: 62
- TAGS: v0.4.0,factions,ai,hud

Spike: docs/spikes/20260708-203517-roadmap-reprioritization-and-juice.md

Enabling task: both the smarter-AI work (20260708-162012) and reticle-by-relation
colouring in the weapons HUD (flagged as an open question in
docs/spikes/20260708-165647-weapons-hud.md) need a notion the game does not have
yet - who is hostile to whom. There is no faction/relation concept today.

Goal: a minimal, lightweight relation model - hostile / neutral / your-own -
sufficient for v0.5.0, not a full faction/reputation system.

Direction (for /plan to break into steps):
- A small component/relation expressing an entity's allegiance (e.g. player,
  enemy, neutral) and a helper to resolve the relation between any two entities.
- Consumers: AI target selection filters to hostiles; HUD reticles/pips colour by
  relation (hostile vs your-own-torpedo vs neutral).
- Keep it deliberately small; a fuller faction system (alliances, reputation) is
  its own future spike if the game ever needs it.


## Steps

- [x] New module `crates/nova_gameplay/src/relations.rs`: an `Allegiance`
      component enum (`Player`, `Enemy`, `Neutral`) and a `Relation` enum
      (`Own`, `Hostile`, `Neutral`), plus a pure
      `relation(Option<&Allegiance>, Option<&Allegiance>) -> Relation`
      (missing component = Neutral; Player vs Enemy = Hostile both ways;
      same combatant allegiance = Own; Neutral never relates strongly, not
      even to itself). Unit-test the full matrix. Register types, export via
      prelude, wire into the gameplay plugin.
- [x] Projectile inheritance: at projectile/torpedo spawn, copy the owner's
      `Allegiance` onto the spawned body (spawn paths that already set
      `ProjectileOwner`), so "your own torpedo" resolves as Own to the
      player and Hostile to its target. Unit tests: spawned bullet and
      torpedo carry the shooter's allegiance; unaligned shooter spawns an
      unaligned bullet.
- [x] Spawn wiring: done via component requires instead of scenario edits -
      `PlayerSpaceshipMarker` requires `Allegiance::Player` and
      `AISpaceshipMarker` requires `Allegiance::Enemy`, so every marked root
      (scenario-spawned or test-spawned) participates with zero extra
      wiring. `nova_scenario` needed no change (plan updated to reality).
- [x] Consumer swap (targeting): the signature-fallback hostility flag in
      `input/targeting.rs` (`Has<AISpaceshipMarker>`, line ~231) becomes
      relation-vs-player == Hostile. Existing signature tests stay green;
      added: a Neutral ship inside signature range is never auto-acquired.
- [x] Consumer swap (HUD): reticle tint by relation in
      `hud/torpedo_target.rs` - hostile (red) vs own(-torpedo) (green) vs
      neutral (white) via `ImageNode.color`; test drives all three.
- [x] Verify: cargo fmt, cargo check --workspace, ran relations (1),
      input:: incl. targeting (36+23), hud torpedo_target (11),
      turret_section (10), torpedo_section (21) - all green. Skipped
      honestly per user instruction: full local suite and clippy (CI runs
      the suite).

## Notes

- Relevant files: crates/nova_gameplay/src/relations.rs (new),
  input/targeting.rs, hud/torpedo_target.rs,
  crates/nova_scenario/src/objects/spaceship.rs,
  sections/projectile_hooks.rs (ProjectileOwner),
  sections/torpedo_section/, sections/turret_section.rs (spawn paths).
- Decision: projectiles COPY the owner's allegiance at spawn (survives owner
  death, keeps queries simple) instead of resolving through ProjectileOwner
  at read time.
- Deliberately NOT here: AI target selection over relations (20260709-225727)
  and any reputation/alliance system (future spike).
- Asteroids and unowned bodies carry no Allegiance and resolve Neutral.

## Resolution (20260709)

Shipped `relations.rs` (Allegiance component + Relation enum + pure
`relation` resolver over `Option<&Allegiance>`), allegiance-by-requirement
on the two ship markers, allegiance copy-at-spawn in both projectile paths,
the targeting hostility swap, and the reticle relation tint. 7 new tests
across 5 modules; all touched modules green.

Decisions worth recording:
- Component requires (`#[require(Allegiance = ...)]` on the markers) beat
  scenario spawn wiring: less code, and every test world that spawns a
  marker gets the relation model for free - the existing targeting tests
  passed unchanged after the swap for exactly this reason.
- `Neutral vs Neutral = Relation::Neutral`, not `Own`: two asteroids are
  not allies; only combatant sides (Player/Enemy) relate strongly.
- Copy-at-spawn over ProjectileOwner-lookup for projectile allegiance:
  survives shooter death and keeps consumers single-query.

Difficulties: none of substance; the first `tatr new` collision aside (spike
phase), the cycle was clean. Reflection: reading the two prior retros first
paid off - test filters ran one module prefix at a time, and the invariant
tests (neutral never auto-acquired; formation of relation matrix) encode
the task's complaint directly.
