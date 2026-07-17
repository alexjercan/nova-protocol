# Retro: turret joint-tree core (merged 215742 + 215804 + 215835)

Landed as one commit (606c3576) on `turret-arbitrary-joints`.

## What went well

- **Reading the aim code before designing paid off twice.** The single most
  valuable spike finding - that the old yaw/pitch systems were ALREADY a
  per-joint local-frame decomposition - meant the generalization was "lift the
  primitive into a loop", not "invent IK". And discovering `SmoothLookRotation`
  never reads `Transform` let the base+rotator pair collapse to one entity per
  joint, halving the entity count and simplifying the sync.
- **Delegate the bulk, hold the risk.** A precise written spec (PLAN.md) let a
  subagent execute a ~1000-line refactor to cargo-green with tests, while the
  hard part (the CCD math, behavioral-parity test design) stayed under direct
  review. Reviewing the aim solver line-by-line + trusting the convergence test
  over theta-equality was the right division of labor.
- **Behavioral parity beat exact parity.** The old pitch formula had
  hand-tuned sign quirks; chasing byte-parity would have been a tar pit. Pinning
  "muzzle forward points at target within N degrees" is both simpler and a
  stronger guarantee.

## What went wrong / bugs

- **Generated artifacts need reviewing too.** The first cut serialized
  `speed: 3.1415927` onto every joint - fixed nodes included - because it was a
  bare `f32` with `serde(default)`. Green tests hid it (parity only checks
  builder==committed, not readability). Caught by reading the generated RON with
  a modder's eye; fixed with `skip_serializing_if`. Lesson: when a refactor
  changes an AUTHORED/GENERATED schema, read the generated file, not just the
  code.
- **Docs drift is silent.** The dev wiki's turret schema and the CHANGELOG both
  needed the new shape; nothing failed without them. The `keep-docs-in-sync`
  ledger lesson applied exactly - the fix belonged in this task, not a follow-up.

## What to do differently

- The five seeded tasks had an inseparable core (1-3): splitting them would have
  meant throwaway shim code. Surfacing that to the user BEFORE writing shims
  (and merging to one commit) was the right move - a spike that seeds coarse
  tasks should expect the plan phase to re-cut boundaries, and flow should ask
  rather than grind. See ledger `inseparable-seeded-tasks-remerge`.

## Follow-ups still open

- 20260717-215857 multi-muzzle firing (the single-muzzle core warns on >1 muzzle
  today).
- 20260717-215920 editor/lint + joint-tree well-formedness lint.
