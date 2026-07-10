# Shot-down torpedo dies: destroyed body section kills the torpedo root without full blast

- STATUS: IN_PROGRESS
- PRIORITY: 74
- TAGS: v0.4.0,torpedo,bug,ai


## Goal

User report (20260710): destroying a torpedo's body still lets it explode.
The torpedo root (collider-less, carries TorpedoBlast + guidance +
torpedo_detonate_system) never learns its child sections died: the husk
keeps flying, armed, and detonates on proximity. Shooting down a torpedo
must actually kill it - despawn the root WITHOUT triggering the full
warhead blast (defeating the blast is the point of point defense). This
makes torpedoes meaningfully easier to destroy: one killed section (1 HP
each) ends the threat.

## Steps

- [x] Observer in sections/torpedo_section (on Add of HealthZeroMarker):
      when the dying entity is a child section of a TorpedoProjectileMarker
      root, despawn the whole torpedo root. Any body section dying kills
      the torpedo - controller and thruster are both vital on ordnance.
- [x] No warhead blast on a shot-down kill: the root despawns without
      spawning blast_damage; reuse/keep whatever section-level explosion
      juice already fires for the dying section so the kill still reads.
- [x] Tests: physics-level (integrity harness) - damage a torpedo body
      section to zero, assert the torpedo ROOT despawns and no
      BlastDamage entity spawns; regression - a healthy torpedo still
      detonates on its target (existing armed-detonate test stays green).
- [x] Verify: cargo fmt, cargo check --workspace, torpedo_section:: tests
      (skip full local suite per user instruction; report skips honestly).

## Notes

- Relevant: sections/torpedo_section/mod.rs (spawn: children
  torpedo_controller/torpedo_thruster base_sections, 1 HP each),
  projectile.rs (torpedo_detonate_system, blast on proximity),
  integrity/glue.rs (HealthZeroMarker pipeline).
- Related: 20260706-212910 (asteroid husk lingers) - same husk family;
  this fixes the torpedo instance only.
- Pairs with 20260709-225733 (AI PDC) and the player's anti-torpedo
  gunnery: both only matter if a hit torpedo actually dies.

## Resolution (20260710)

Shipped `on_torpedo_body_destroyed` (observer on Add<HealthZeroMarker>):
any dead child section of a TorpedoProjectileMarker root try_despawns the
whole root - the husk can no longer fly on and detonate. No blast_damage
spawns on a shoot-down kill (asserted by test through the real
HealthPlugin damage pipeline); only torpedo_detonate_system explodes.
A guard test pins that ship sections dying do NOT despawn their ship.
3 new tests; full crate suite 221/221 green this once, fmt + check clean.
Skipped per user instruction: clippy.

Note for polish (not filed as a task yet): the instant despawn happens at
the HealthZeroMarker stage, before the integrity destroy pipeline spawns
its section debris, so a shot-down torpedo currently vanishes without
much visual fanfare. If playtest wants a kill flash, that is a juice
follow-up.

## Reopened (20260710): live-game panic

User hit a panic in the running game: `Entity despawned ... 1202v4` from
`insert<IntegrityDisabledMarker>` inside avian's collision-event flush.
Root cause: the shoot-down observer despawns the torpedo root (and with it
the dying section) in the SAME command flush where the integrity pipeline
has already queued inserts for that section - those commands then target a
despawned entity and panic.

Fix: two-step kill. The observer only INSERTS `TorpedoShotDownMarker` on
the root (inserting on a live entity is always safe); a
`despawn_shot_down_torpedoes` system does the actual despawn on the next
schedule pass, after the integrity commands have landed; and
`torpedo_detonate_system` excludes marked roots so the warhead cannot fire
in the one-tick gap. Tests updated to drive the marker + despawn pipeline,
plus a regression asserting a marked torpedo does not detonate.
