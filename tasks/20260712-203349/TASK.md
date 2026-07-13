# Committed torpedoes do not auto-steal the aim lock (manual lock only)

- STATUS: CLOSED
- PRIORITY: 28
- TAGS: v0.5.0, targeting, spike

## Closed won't-do (2026-07-12)

Superseded by the sticky-from-acquisition lock (task 20260712-203353). Once a
held lock is not overwritten by the aim picker, a passing torpedo cannot steal
it, so there is no need to exclude torpedoes from auto-acquisition - and the
user explicitly wants torpedoes to stay lockable for point defense (aim/scroll
to one to shoot it down). No code from this task.

## Goal (original, not pursued)

Stop a committed torpedo streaking across the aim ray from AUTO-stealing the
target lock (and resetting the 1.5 s focus dwell). Torpedoes stay MANUALLY
lockable for point defense - you aim at the one you want - but they do not win
the aim-driven cone pick against/into an existing or forming lock.

Direction (see spike): the cone pick (`pick_target` in
`update_spaceship_target_input`, input/targeting.rs) currently runs over ALL
candidates with no ship/transient filter. Exclude committed torpedoes
(`TorpedoProjectileMarker` + `TorpedoTargetChosen`) from the AUTO cone pick's
candidate set, while keeping them in the set that an explicit manual lock /
point-defense can still select. Do NOT exclude beacons/asteroids - the lock is
the GOTO/torpedo designator and those are designated by aiming (beacon scope is
handled separately by task 20260712-203345).

## Notes

- Spike: tasks/20260712-203235/SPIKE.md
  (Part 2, option B2).
- Relevant files: `crates/nova_gameplay/src/input/targeting.rs`
  (`update_spaceship_target_input` cone pick; candidate collection already
  tracks `is_torpedo` + `torpedo_committed`).
- Small, low feel-risk; protects the forming dwell that the sticky-lock task
  (20260712-203353) does not cover on its own.
- Buy-in requested before implementing.
