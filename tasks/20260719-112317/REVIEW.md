# Review: the probe front door (runner CLI)

- TASK: 20260719-112317
- BRANCH: feature/probe-runner-cli

## Round 1

- VERDICT: APPROVE (one MINOR, addressed in-round)

Shared-session caveat: implementer and reviewer are one session; claims
re-derived against artifacts and prior contracts:

- **Both e2e promises kept, with real runs**: the clean front-door run
  (10_playable -> verdict OK, all four artifacts) and the --profile
  composition (08_scenario -> both passes, trace.json rendered into a
  POPULATED top-N section, trace-run.log absent from log_clean's input).
  The --profile e2e was run despite its cold-build cost precisely because
  the pass-2 call site was the one un-unit-tested composition
  (degrade-paths-need-a-forced-failure's sibling: exercise the promised
  path once).
- **Plan adaptations are recorded, not silent**: script-wrapping instead
  of a Rust rewrite (with the revisit owner named: the tooling-inventory
  umbrella 20260718-152304), no --platform web on `run` (timeline and
  invariants are native-only by T2/T3 design), no --export flag (the
  artifacts are the formats). Each carries its reasoning in TASK.md.
- **Process hygiene matches the ledger's hard-won rules**: Xvfb killed by
  recorded PID via a Drop guard (pkill-pattern-matches-own-shell), the
  run timeout kills a hung child rather than wedging the check, the
  RUSTFLAGS frame-pointer flag only ever pairs with the profiling
  profile's cache, and the profiled pass's RUST_LOG override composes
  with a caller's existing RUST_LOG instead of clobbering it (pinned).
- **Env assembly is where orchestration bugs live and it is pure +
  pinned**: recorder/invariants always on, fps strictly opt-in, the
  profiled pass writes trace.json and NEVER the timeline path (asserted -
  the overwrite would have corrupted pass 1's artifact silently).
- **Checks**: 55 tests green; workspace all-targets + wasm clean; the
  wasm stub main landed in the same edit as the bin registration (T5's
  lesson observed).

Findings:

- [x] R1.1 (MINOR) crates/nova_probe/src/bin/probe.rs (ensure_display) -
  the throwaway Xvfb display is hardcoded to :97; two concurrent `probe
  run`s (or a parallel session's Xvfb on :97) collide - the second run's
  X clients land on the first's server or fail. Suggested: derive the
  display number from the process id so concurrent runs get distinct
  servers.
  - Response: fixed in-round - display defaults to :(90 + pid % 10)
    with the collision caveat documented in --display's help line;
    pinned by a unit test asserting the derived display stays in :90-:99.
