# Review: Signature-gated lock - long-range lock only acquires large objects

- TASK: 20260710-195952
- BRANCH: signature-lock

## Round 1

- VERDICT: APPROVE (findings MINOR/NIT, fixed in-round before merge)

Reviewed commit 53fda19 against master with an independent adversarial
pass over the diff and every lock consumer. Sound: torpedo commit is
launch-time (in-flight guidance never re-reads the lock), turret feed
degrades to the ray tier, GOTO tracks the entity not the lock, class
precedence (well beats signature, ship beats stray signature), heat
fallback unaffected (hostiles are intrinsic classes), AI has its own
picker (player-only confirmed), all asteroid spawn paths funnel through
the authored bundle, gate math from the cone origin. Findings:

- [x] R1.1 (MINOR) targeting.rs (gate) - hard per-frame cutoff at ranges
  the ship crosses constantly: a rock at its gate boundary strobes the
  lock, resets the 1.5s focus dwell, and flickers the turret aim tier.
  Fix: range hysteresis - the incumbent lock stays lockable out to a
  factor beyond its gate (the file's SNAP_HYSTERESIS precedent).
  - Response: fixed - TargetingSettings::range_hysteresis (1.15); the
    current lock's gate is widened by it at collection, with a
    truth-table test (incumbent held past the gate, fresh acquisition
    still refused, hysteresis released once truly out).
- [x] R1.2 (MINOR) targeting.rs - a signature below 0.5 gates BELOW the
  unsigned debris floor (and negatives behave as abs), inverting the
  component's meaning. Fix: floor the signed range at unsigned_lock_range.
  - Response: fixed - signed range floors at the unsigned range; test
    case pins LockSignature(0.0) at the debris floor.
- [x] R1.3 (MINOR) targeting.rs - committed torpedoes at full 20km range
  is the one real scanner-fiction deviation: the smallest object in the
  game was the only small thing visible at full range. Point defense is
  covered far below that (AI launch range 1000u, heat fallback 550u).
  Fix: a generous fixed torpedo lock range instead of the full-range
  exemption.
  - Response: fixed - TargetingSettings::torpedo_lock_range (2500u,
    covering every real PD engagement with margin); test updated to pin
    a committed torpedo lockable at 2000u and not at 5000u. Flagged for
    playtest retune.
- [x] R1.4 (NIT) targeting.rs - TARGETING_MAX_RANGE's comment still
  promises "designatable from across the play area" for everything,
  documenting the losing side of the two conflicting 20260710 reports.
  - Response: fixed - comment now scopes the constant to the intrinsic
    classes' ceiling, with signatures gating the rest.

Verification of the in-round fixes: 30 targeting tests green (3 new
cases), input module green, fmt + check --workspace --examples clean.
