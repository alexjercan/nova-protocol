# Review: Weapon-section one-shot sounds (dry_fire, torpedo launch)

- TASK: 20260717-101624
- BRANCH: task-20260717-101624-weapon-sounds

Reviewed the committed diff (894350b8) with fresh eyes plus an independent
out-of-context pass. Load-bearing claims re-verified independently:

- Silent-regression sweep: zero mod/webmod/example content defines its own
  Turret/Torpedo sections (all reference base prototypes by id, which author
  every weapon sound via gen_content); the examples clone catalog sections;
  the only bare `TurretSectionConfig` literals outside the family's own tests
  are inside `#[cfg(test)]` (nova_scenario spaceship.rs:336+). No shipped
  content fires silently.
- The single PRODUCTION torpedo projectile spawn includes
  `TorpedoSectionSpawnerEntity(**spawner)` (torpedo_section/mod.rs:641 block);
  every other TorpedoProjectileMarker spawn is in `mod tests` (post-:771). The
  launch observer's query path is complete.
- Dry-fire latch advances for UNAUTHORED turrets too (`*was = dry` outside the
  authored branch), so authorship cannot replay stale edges.
- Suites: nova_gameplay lib 535, content gates 4/4 (parity proves the
  regenerated content matches the builders; lint proves the new self:// sound
  refs are declared resources), workspace all-targets check clean.

Independent pass confirmed all of the above plus test quality (the old
bank-fallback test was correctly REPLACED by the authored-or-silent pair with
a real delivery guard; gating tests still gate on hot/player/ammo) - and found
the one real problem:

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) stale rustdoc claims the deleted bank fallback still exists,
  in three places: turret_section.rs `fire_sound` field doc ("`None` falls back
  to the global [`WorldSfx::TurretFire`] cue"), turret_section.rs
  `TurretSectionFireSound` component doc ("preferred over the global bank cue /
  the bank default"), and the audio.rs module header ("a turret round spawned ->
  `TurretFire`; a torpedo spawned -> `TorpedoLaunch`" - those keys no longer
  exist). Docs must match what the code does: these cues are authored-or-silent
  now.
  - Fix: reword all three to the authored-or-silent model (base content authors
    the defaults; no bank mention).
  - Response: Done - all three sites reworded to authored-or-silent (fire_sound
    field doc names the gen_content-authored base default; the component doc
    drops the bank-default claim; the module header names the section-owned
    sounds per cue). `cargo check -p nova_gameplay` clean.

## Round 2

- VERDICT: APPROVE

R1.1 verified resolved: greps for "falls back", "bank default" and the deleted
key names in rustdoc come back clean; the three sites now describe
authored-or-silent and the module header names the section-owned sounds. No new
findings; the implementation findings from round 1 were already all green.
