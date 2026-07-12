# Unified cone target list + universal sticky lock (absorbs: cyclable nav bodies)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0, targeting, navigation, spike

## Goal

One available-targets list, one lock, per the user's model (steer 2026-07-12,
spike 20260712-215733): the list holds EVERY lockable body inside a cone
around the aim ray - combat (hostile ships, committed torpedoes) AND nav
(signed asteroids, beacons, wells) AND friendly ships - ranked angle-to-aim
then distance. With no lock, the computer auto-picks the best entry: hostiles
first from anywhere in the list, else the best non-hostile entry inside the
tight pick cone (the 550 m hostile signature fallback stays for an empty
list). A held lock of ANY class never changes on its own: only CTRL+scroll
(next entry), death, or the range gate moves it. Unsigned debris stays out by
construction (no LockSignature at range).

## Steps

- [ ] Decouple edge indicators first: emit a new `HostileContacts` component
      (all-directions hostile combat targets, from the same collection pass
      in `update_spaceship_target_input`) on the ship root; switch
      hud/edge_indicators.rs from `AvailableTargets` to it (its committed
      torpedo query stays). Test: a hostile behind the player still gets an
      edge arrow when it is not in the cone list.
- [ ] Add `TARGETING_LIST_CONE_HALF_ANGLE_DEG` (~50.0, feel-knob const next
      to `TARGETING_CONE_HALF_ANGLE_DEG` at targeting.rs:128) and change
      list membership: ALL collected candidates (any class and hostility)
      whose bearing is inside that cone; generalize `rank_combat_targets`
      (targeting.rs:539) to `rank_targets` (same angle-then-distance rule).
      The 5-cap and pinned stable-order rules in `maintain_candidates`
      stay.
- [ ] Change the lock-membership rule in `maintain_candidates`
      (targeting.rs:593-598): the current lock stays an entry while it is
      still a COLLECTED candidate (in range), even when outside the list
      cone - the reticle target must never vanish from its own list, and a
      cycle press must be able to step off an out-of-cone lock.
- [ ] Universal stickiness: drop the `is_combat_target` condition from the
      `held` gate (targeting.rs:483-487). Update the comment block above it:
      aim re-designation of nav targets is removed BY DESIGN (user steer
      2026-07-12); re-designation = cycling, since nav bodies are now in the
      list.
- [ ] Replace the auto-pick (targeting.rs:489-511): no lock -> first HOSTILE
      entry of the list (rank order, anywhere in the wide cone), else the
      best NON-hostile entry within the tight 18 deg pick cone (friendly
      ships and nav bodies designate by aiming), else
      `pick_signature_target` unchanged. The class asymmetry is the spike's
      cruise-noise guard: threats auto-acquire, rocks only when aimed at.
- [ ] Update/extend the targeting tests: a nav (asteroid) lock is sticky
      against aim wander; CTRL+scroll reaches a nav body in the cone; a
      hostile off-aim but in the wide cone auto-locks; an asteroid 30 deg
      off-aim does NOT auto-lock (outside tight cone) while one under the
      crosshair does; a friendly ship under the crosshair locks; an
      out-of-cone lock stays held and stays in entries; behind-player
      hostile is in `HostileContacts` but not in entries.
- [ ] Verify hud/target_candidates.rs brackets and hint rows render nav
      entries sanely (should be no code change; eyeball a scenario with
      04_asteroids).
- [ ] cargo fmt + cargo check, run the touched test modules (targeting +
      hud) - full suite runs in CI.

## Notes

- Spikes: docs/spikes/20260712-215733-unified-target-computer.md (the model,
  auto-pick policy, cone knob, open questions);
  docs/spikes/20260712-215256-combat-travel-lock-separation.md (Part A -
  original motivation: flick to a far body you cannot pixel-aim).
- Depends on: 20260712-215957 (componentized TargetLock/AvailableTargets).
- 2026-07-12 steer: this task originally implemented option A1 from the
  combat/travel spike - nav bodies as NON-sticky, combat-first cycle entries,
  keeping aim re-designation. The user's newer steer supersedes A1: keep it
  simple, one lock for both classes, sticky for both, cone membership is the
  only gate. The combat/travel mode toggle stays a future direction recorded
  in that spike doc (not a seeded task; see its Next steps/Addendum).
- Behind-you threats are deliberately NOT cyclable under the strict cone
  (spike open question, default per user steer): edge indicators still warn
  via `HostileContacts`, and turning to face brings them into the list.
  Revisit from playtest.
- Deferred playtest knob (spike, "Combat vs travel separation"): gate
  auto-pick on the Turret camera view (RMB held) so cruising stays quiet.
  NOT in this task's scope; fast-follow if the reticle still churns.
- Clutter guard in dense fields (04_asteroids): the wide-cone gate + 5-cap
  is the v0.5 answer; signature thresholds / combat-reserved slots only if
  playtest shows crowding.
- Relevant code: `update_spaceship_target_input`, `rank_combat_targets`,
  `maintain_candidates`, the `held` sticky gate, `TARGET_CANDIDATE_COUNT`,
  `pinned_until` (now order-freezing only - stickiness no longer needs the
  pin), LockSignature range model (unchanged).
- Playtest: does cycling-to-redesignate feel OK for GOTO; explicit unlock
  input wanted; wide-cone half-angle value.
