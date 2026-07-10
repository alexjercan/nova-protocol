# Review: Shot-down torpedo dies without its blast

- TASK: 20260710-003734
- BRANCH: fix/torpedo-shootdown

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...fix/torpedo-shootdown` against TASK.md; full
nova_gameplay suite on the branch: 221/221 green. The diagnosis matches
the code (the root carries the fuze and no Health; children die through
the normal pipeline and nothing propagated the kill), and the fix is the
minimal correct seam: an observer at the HealthZeroMarker stage, scoped
by the torpedo marker on the PARENT, with try_despawn for the
both-sections-die-same-burst race, and deliberately no blast_damage - the
design rationale is documented at the observer. The test trio is exactly
right: the unit kill, the real-pipeline quiet-death (asserting zero
BlastDamageMarker entities through HealthPlugin damage propagation), and
the non-torpedo guard pinning that ship sections dying do NOT despawn
ships. The suppressed-debris polish gap is honestly recorded in the
Resolution rather than papered over. Existing armed-detonation regression
tests stay green, so a healthy torpedo still explodes on its target.
No findings.

## Round 2 (reopened: live-game panic, branch fix/torpedo-shootdown-defer)

- VERDICT: APPROVE

Reviewed `git diff master...fix/torpedo-shootdown-defer` (commit 51dba63)
against the reopened TASK.md. The round-1 fix despawned the torpedo root
inside the HealthZeroMarker observer, in the same command flush where the
integrity pipeline had already queued inserts (IntegrityDisabledMarker)
for the dying section; those commands then hit a despawned entity and
panicked inside avian's collision-event flush. The user's crash trace
matches this diagnosis exactly.

The two-step kill is the right shape and I verified the scheduling
argument holds:

- The observer now only try_inserts TorpedoShotDownMarker on the root
  (safe on a live entity, no-op on a dying one), and
  despawn_shot_down_torpedoes does the try_despawn from Update.
- The panicking command wave is queued during the FixedMain physics
  flush, which runs BEFORE Update in the same frame, so every queued
  command for the dying section has landed by the time the despawn
  system runs. Follow-up integrity reactions are observer-driven and
  land in the same flush cascade; post-despawn systems no longer see
  the section in their queries. No remaining ordering hole found.
- torpedo_detonate_system takes Without<TorpedoShotDownMarker>, and
  despawn_shot_down_torpedoes is chained ahead of it, so the fuze
  cannot fire in the one-pass gap. Pinned by
  a_shot_down_torpedo_cannot_detonate_in_the_removal_gap.
- the_kill_does_not_race_commands_queued_for_the_dying_section
  reproduces the crash pattern (same-flush insert on the section after
  the observer ran) and fails with a panic on the round-1 code.
- Existing round-1 tests were updated to drive the marker + despawn
  pipeline rather than weakened; the quiet-death (zero
  BlastDamageMarker) and non-torpedo guard assertions are intact.
- Marker derive set and registration match the module's conventions.

Checks on the branch: cargo fmt --check clean, cargo check -p
nova_gameplay clean, torpedo-filtered suite 46/46 green (full suite
deferred to CI per repo policy).

One non-blocking observation, left to the implementer's discretion:
in the marker-to-despawn gap the torpedo body still simulates for up
to one frame (it can still collide and deal contact impact). That
window existed implicitly in round 1 too and is not the reported bug.

No findings.
