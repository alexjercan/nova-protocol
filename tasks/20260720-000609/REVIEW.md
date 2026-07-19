# Review: harness completion protocol (S1)

- TASK: 20260720-000609
- BRANCH: feature/harness-completion-adoption (+ bcs feature/harness-completion, shipped v0.19.3)
- ROUND: 1

## What I tried to break

- **Single-collector regression** (the whole fleet's stock examples):
  parity is by construction (empty pending set -> Success, same frame as
  the old direct write) and pinned by a dedicated test; e2e A ran a
  real single-collector example unchanged.
- **The headline race**: e2e B is the exact failing case from the field
  (scenario, default windows) - full 900-frame capture, log showing the
  autopilot done with the capture still pending. Not a synthetic proof.
- **Stalled-script masking**: self_completing turns a runway expiry into
  an ERROR exit naming the script; broadside's belt guard stays. The
  laggard e2e (D) proved the deadline names BOTH pending collectors and
  probe fails the run with artifacts intact.
- **Feature-gate trap**: completion lives ungated at the bcs crate root
  precisely because nova's capture compiles featureless (wasm perf_web);
  both feature configurations tested in bcs.
- **Graph splitting**: the five-manifest path-dep dance was verified by
  trait-resolution itself (a missed manifest fails to compile, as it did
  once); the restore is total - the lock's source line reads the pushed
  tag commit.

## Findings

- R1.1 (NIT, accepted): registration name collisions across crates rely
  on convention (string names); a collision warns at registration. Fine
  at this scale.
- R1.2 (recorded for S2): e2e B's capture tail measures post-script idle
  frames - correct per S1's scope, and S2's scene looping owns it.
- R1.3 (NIT): the laggard e2e named autopilot AND capture at the 10s
  deadline (startup ate the autopilot's margin) - accurate at that
  instant, but deadline defaults deserve a look if false laggards ever
  confuse a report reader.

## Verdict

APPROVE - land.
