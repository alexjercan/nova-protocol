# Fire gating + combat unlock: LMB acquires when unlocked, fires when locked; dedicated unlock key

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.5.0, targeting, input, spike

## Goal

Make firing deliberate per the two-slot model (spike 20260712-222610): with
a CombatLock the trigger fires as today; without one it does NOT fire - it
acquires the combat lock instead. CTRL free-aim stays the deliberate
unlocked-fire path. Add a dedicated key that clears the active view's lock.

## Steps

- [ ] Gate the turret trigger in its observer (`on_turret_input` Start,
      player.rs:1054; bindings stay scenario-authored via
      `SpaceshipTurretInputBinding` - no data change): CombatLock present ->
      fire (unchanged three-tier feed); absent -> acquire the best enemy by
      the `HostileContacts` ordering (cone-first; no-op when none) and do
      NOT fire. Applies in every view, keyboard and pad identically.
- [ ] CTRL free-aim bypass: free-aim held -> the trigger fires at the
      camera ray regardless of lock (existing behavior, now the ONLY
      unlocked fire path). Verify against the free-aim action confirmed in
      20260712-223034.
- [ ] Torpedo launch (player.rs:1128): keep existing semantics - lock ->
      guided commit, no lock -> dumb-fire (spike default, flagged for
      playtest). Add a comment citing the spike so the asymmetry with
      turrets is documented as chosen, not accidental.
- [ ] Unlock key: new input action bound to X (verify unbound at
      implementation; spike placeholder), clearing the ACTIVE view's lock -
      Turret view -> CombatLock, Normal/FreeLook -> TravelLock (spike
      default, flagged). Pause-gated like the cycle observers. Pad
      candidate: East/B - verify against the binding table
      (player.rs:558-692) before choosing.
- [ ] HUD hint rows: unlock key hint; trigger hint reflects state (fire vs
      acquire). Feedback when a trigger press acquires instead of firing:
      reuse the lock-acquired affordance (reticle snap) - no new audio/VFX
      scope.
- [ ] Tests (state-per-step): trigger with no CombatLock acquires and does
      not spawn bullets; second press fires; CTRL+trigger fires with no
      lock; trigger acquires nothing when no enemies (and does not fire);
      unlock clears CombatLock in Turret view and TravelLock in Normal
      view; unlock while empty no-ops; pad trigger follows the same gate.
- [ ] cargo fmt + cargo check + run targeting/input test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md.
- Depends on: 20260712-223035 (slots must exist).
- Muscle-memory risk is real: this changes what LMB does with no lock.
  The spike's rationale: seed-on-raise means the common case is already
  locked when the player wants to shoot; CTRL covers panic fire. Playtest
  verdicts go on this task.
