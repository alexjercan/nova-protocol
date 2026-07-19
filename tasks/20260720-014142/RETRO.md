# Retro: loop gate readiness (field crash, three layers)

- TASK: 20260720-014142
- BRANCH: fix/loop-gate-readiness (landed f2e5452a); bcs v0.19.5 (30d1bef)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs)

## What went well

- The field found exactly what the review had flagged: S2's R1.1 said
  "playable never looped on this host; its loop path is exercised only
  by scenario" - the user's real-GPU run cashed that NIT within the
  hour. Recording untested paths as named findings made the crash a
  ten-minute diagnosis instead of a mystery.
- Each e2e round peeled ONE honest layer (gate timing -> deadline
  semantics -> dead-cycle tail) and each fix landed at its owning site:
  the examples own readiness, the scripts own enforcement scope, bcs
  owns the looping regime's finish. No fix papered over another layer.
- The third bcs patch release in ~24h ran on rails: the upstream rhythm
  (branch, lib-test both configs, version+CHANGELOG, push+tag, pin bump,
  retest on the published tag) is now genuinely cheap.

## What went wrong

- ScenarioLoaded-means-ready was ASSUMED in S2 and only scenario's
  coincidental stage-1 wait masked it through S2's e2e - the same
  seeding-async fact had ALREADY been learned in S2 (its own third e2e
  round!) but was fixed locally in one script instead of being read as
  a property of the SIGNAL. A lesson learned narrowly gets relearned
  expensively.
- The 5000-frame e2e window ignored deadline arithmetic (frames /
  Xvfb-rate vs 120s), spending one full e2e round on a self-inflicted
  deadline verdict.

## What to improve next time

- When an e2e failure is fixed by tolerating a signal's timing, ask
  "what does this signal actually GUARANTEE?" and fix the consumers of
  the signal, not the one consumer that crashed.
- fps e2e windows get sized by arithmetic (frames / expected rate vs
  deadline + probe timeout) before running, not after.

## Action items

- [x] bcs v0.19.5 shipped (mid-cycle done).
- [ ] R1.2 carried: forced-loop e2e knob (NOVA_PERF_FRAMES=2000)
      documented in the close-out; automate if the loop path regresses
      again.
