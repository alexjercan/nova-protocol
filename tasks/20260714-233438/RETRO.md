# Retro: Switch web build to bevy/webgpu and un-gate hanabi on wasm

- TASK: 20260714-233438
- BRANCH: feat/wasm-webgpu-particles
- REVIEW ROUNDS: 1 (APPROVE, one MINOR + one NIT)

## What went well

- Verified the load-bearing fact instead of assuming it: that bevy's `webgpu`
  overrides `webgl2` when both are enabled was checked twice (bevy docs via web
  search at plan time, then `cargo tree --target wasm32` at implement + review
  time showing webgpu on wasm and NOT on native). The whole approach rests on
  that one fact, so it earned two independent confirmations.
- Applied `verify-ci-triggers-before-claiming-coverage` proactively: read the
  workflows first, saw CI builds native only and deploy is `workflow_dispatch`,
  concluded the un-gated wasm code has no automated compile gate, and ran the real
  `trunk build` (5m) rather than trusting a green native CI. It compiled hanabi for
  wasm32 cleanly - the single meaningful verification of the change.
- Grepped the code to check the spike's claims rather than trusting them, which
  caught that the "thruster plume" is a shader (`ThrusterExhaustConfig`), not
  hanabi, and already rendered on the web. Corrected the CHANGELOG/NOTES/task so
  the shipped record is accurate.

## What went wrong

- R1.2 (NIT): reflowing the `juice.rs` doc comment to drop the stale "wasm-blocked"
  clause left one line at 101 cols, inconsistent with the file's ~80-col wrap. Root
  cause: edited the sentence in place without re-wrapping the surrounding block to
  the file's column width. Trivial, caught in review.
- The spike carried an inaccuracy (thruster plume listed as a gated hanabi effect)
  into the plan. Root cause: the spike enumerated effects from the task's prose
  without grepping which are actually `bevy_hanabi`. It cost only a correction here
  because the implement step grepped, but a cycle that trusted the list would have
  shipped a wrong CHANGELOG claim.

## What to improve next time

- When editing an existing wrapped comment, re-wrap the whole block to the file's
  column width, not just the touched line.
- Treat a spike's enumerated list of mechanisms/effects as unverified until the
  implementing cycle greps each item against the code.

## Action items

- [x] Bumped `verify-ci-triggers-before-claiming-coverage` to x2 in the ledger.
- [x] Added `spike-list-needs-code-check` and a positive
  `target-scoped-feature-flips-wasm-backend` lesson to the ledger.
- Runtime particle eyeball in a WebGPU browser is deferred to the paired gate task
  20260714-233443 (its verify step, strengthened during its work cycle) - not a new
  task, tracked in REVIEW.md R1.1.
