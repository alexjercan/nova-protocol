# Retro: Mod dependencies

- TASK: 20260715-142931
- BRANCH: feature/mod-dependencies (landed on master as c04a8379)
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES with one MAJOR + minors, R2 APPROVE)

## What went well

- Staging one cohesive feature into SIX per-part commits (engine-free helper ->
  merge order -> enable/disable -> install -> UI -> docs) kept each part
  independently testable and made the out-of-context review navigable. Every
  part landed green before the next.
- The engine-free helper (`nova_mod_format::deps`) as the foundation paid off:
  the hard algorithm (Kahn's topological sort, transitive closure) got 9 fast
  unit tests with zero bevy, and the four consumers (merge, enable, install, UI)
  became thin, correct wiring over a proven core.
- The out-of-context review re-derived Kahn's direction, the merge-overlay
  direction and the cycle-termination argument independently rather than trusting
  the summary - and caught the one real gap.

## What went wrong

- The MAJOR was a DOCUMENTATION overclaim, not a logic bug: NOTES implied the
  dependency-SET install is atomic when it is only per-mod (the mod and its deps
  download in parallel with no join, so an async dep-download failure leaves the
  dependent installed). Root cause: I wrote the "why" notes from the INTENT
  ("pull deps first") without tracing the async FAILURE path. The behavior is
  fine and surfaced (Failed job + enable-time warning); the words were wrong.
- The branch base was RED: a parallel Gauntlet-playable change landed a new
  bundle description on master without updating the `mod_cache_install` fixture
  that asserts it. Inherited someone else's incomplete sweep; realigned it on the
  branch (a small, unrelated fix) to keep the tree green.
- Per-part commits shipped UNFORMATTED (fmt only ran at Part 6). Harmless for a
  squash-merge, but `lint-gate-is-the-last-step` again: I ran tests per part, not
  fmt.

## What to improve next time

- When documenting a concurrent/staged flow, trace the async FAILURE path and
  state the atomicity boundary explicitly, not just the happy-path intent.
- Run `cargo fmt` (not only tests) before each commit, or accept that the branch
  tip is the only formatted state - and never claim green before an fmt pass.

## Action items

- [x] LESSONS.md: new `document-the-async-failure-path`; bumped
  `out-of-context-review-pass`, `lint-gate-is-the-last-step`; new
  `sibling-change-leaves-stale-fixture`.
- Follow-ups recorded in TASK.md close-out (not filed as tasks - speculative):
  atomic dependency-SET install; hidden-mod dependency enable; version
  constraints.
