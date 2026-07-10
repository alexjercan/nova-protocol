# Retro: Camera handback blend

- TASK: 20260710-222517
- BRANCH: fix/camera-handback-blend (squashed to master)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 2 NIT, fixed in-round)

## What went well

- Root-causing before designing: reading WHY mode switches are smooth
  (they re-seed from the current output) and why the disengage cannot do
  that (the PD contract) split the problem into two consumers of one
  discontinuity - the fix then wrote itself as "bridge only the camera's
  reading". Verifying anchor_rot had exactly one consumer before
  building on it made the blast radius provably zero.
- The reviewer EMPIRICALLY tested the despawn-flush claim (scratch app,
  reverted) instead of trusting the guard comment - and the comment was
  indeed false. Worth adopting: observer-semantics claims (what fires
  during despawn, what a Remove observer can still see) are cheap to
  verify with a five-line scratch test and wrong often enough.
- Quat::angle_between epsilon lesson captured in-code: acos amplifies
  float noise to ~1e-3 rad for "equal" quats; assertions on quats need
  epsilons sized to that model, not to intuition.

## What went wrong

- The "or it is despawning" guard comment asserted behavior I never
  tested (a Remove observer's query failing during despawn - it does
  not). The code survived by an accident of the teardown path. Same
  lesson class as the eta-degradation finding two cycles ago: a comment
  that claims a runtime behavior is a test obligation, not prose.

## What to improve next time

- When writing "this observer bails when X despawns", write the
  five-line scratch test first or do not write the comment.

## Action items

- [x] Fixed in-round (honest comment + defensive blend clear on
  controller re-add).
- [ ] Pre-existing, noted by review, not filed as it may fold into the
  diegetic-status overhaul: FreeLook-to-Normal after an autopilot
  disengage still pops (the dormant normal rig was re-seeded to the hull
  while FreeLook drove). If playtests surface it, file it then.
