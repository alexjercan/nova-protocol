# Retro: ch5 raid playtest tuning

- TASK: 20260723-200643
- BRANCH: feature/ch5-raid-tuning
- REVIEW ROUNDS: 1 (APPROVE, out-of-context; one MINOR, a disclosed playtest risk)

See TASK.md Outcome for what/why; this is process only.

## What went well

- Investigating the ENGINE model before touching the content is what made this
  correct. The user's literal suggestion ("add a controller so it can RCS")
  would have produced an armed AI base that CHASES the player - because I first
  read the gravity SOI formula (`8 * radius`), the "only piloted ships feel
  gravity" rule, AND the `AIBehaviorState::Engage` = chase behaviour, I could see
  the trap and find the real lever (a tight `AILeash` bounds the chase). Reading
  the AI behaviour state machine + the physics before writing beat guessing.
- Surfaced the divergence at the gate instead of silently overriding the user:
  presented thrusterless-vs-RCS-vs-passive, and the user's note ("place it such
  that RCS would work, safe distance") steered the design. The gate earned its
  keep on a change that contradicted the user's stated mechanism.
- The lint again caught the structural risks (new thruster mounts valid, R not a
  reserved flight key) in one fast pass, before the rig - the
  `lint-is-the-fast-oracle-for-new-scenarios` lesson from last cycle held.
- Geometry was computed, not eyeballed: the base-to-planetoid distances and the
  residual accel (~0.08 u/s^2) were calculated in the same script that edited the
  planetoids, and the reviewer independently reproduced the numbers.

## What went wrong

- The ch5 rig's bundle-version pin (`contains "1.10.0"`) broke when I bumped to
  1.11.0 - a fixture pin in the SAME file as the change, and the SECOND time this
  exact class bit this run (last cycle it was the ch4 rig's sell-chain pin). Root
  cause: a version-string assertion lives far from the `meta.version` edit even
  within one file, so a bump silently invalidates it until the test runs.
- One MINOR from review I did not foresee: the engine's `recently_damaged` tether
  override lets a shot ship exceed its leash to defend itself - and the base is
  the torpedo target, so the leash does not fully bound it under fire. It is
  honestly disclosed as a playtest item (the Outcome already listed the levers),
  but I had reasoned "tight leash => cannot chase" without checking the override
  path. Root cause: I read the leash's normal path (`leash_exceeded`) but not the
  damage-override branch a few lines below it.

## What to improve next time

- Any bundle/version bump: grep the test tree for the old version STRING in the
  same change (`grep -rn '"1.10.0"' crates/`) - this pin has now bitten twice.
- When relying on a bounding mechanism (a leash, a cap, a guard) for correctness,
  read its OVERRIDE/exception branches too, not just the happy path - the
  `recently_damaged` override is exactly the edge that matters for a base that
  gets shot.

## Action items

- [x] Fixed the bundle-version pin to 1.11.0.
- [x] R1.1 accepted as a disclosed playtest item; added to the umbrella Manual
  acceptance checklist (confirm the base does not walk under fire).
- [x] Lessons ledger: bumped `fixture-pin-far-from-diff` (version-string-in-test
  variant) and added `read-the-override-branch-of-a-bounding-mechanism`.
