# Review: profiled pass (chrome-trace top-N + perf_trace + perf-profile.sh)

- TASK: 20260719-112253
- BRANCH: feature/probe-profiling

## Round 1

- VERDICT: APPROVE

Shared-session caveat: implementer and reviewer are one session; the
load-bearing claims were re-derived or settled EMPIRICALLY, not read off
the diff:

- **The two field facts are evidence-backed, not theorized.** (1) System
  spans killed by the game's own `bevy_ecs=warn` filter: proven by a
  minimal throwaway probe app (spans recorded) vs the real app (zero spans
  in a 38 MB trace), then the fix proven by re-run (456k system spans with
  `RUST_LOG=bevy_ecs=info`). Three source-reading theories were tried and
  discarded first - the review accepts the empirical chain, which is
  stronger. (2) The samply perms failure was reproduced (EXIT=1 under
  set -e), hardened, and the graceful skip re-proven (EXIT=0, table +
  trace intact).
- **Parser arithmetic pinned by literals**: the fixture's expected values
  are hand-computed (2 calls x 1000 us = 2.0 ms, 80/20 shares) and include
  the QUOTED name form the real trace emits (`name="game::beta"`) plus
  nested non-system spans exercising the per-tid stack pairing. A unit or
  pairing bug cannot pass these.
- **Honesty of the numbers**: per-call + share-of-system-time only; the
  no-frame-span limitation is stated in the module docs, the table header
  itself, and TASK.md - a pasted table cannot silently overclaim per-frame
  meaning. The two-pass rule is enforced structurally (separate script,
  separate build features) not just documented.
- **Spec vs diff**: all 8 steps re-verified clause by clause at tick time
  (individually, per the fresh half-ticked lesson). The e2e headline
  numbers in TASK.md match the committed top-systems output.
- **Would-it-fail audit**: quote-trim (fixture), stack pairing (nested
  span), X handling, top-N cut note, malformed reject, empty trace - each
  has an assertion that fails with the mechanism deleted. 42 tests pass;
  workspace all-targets + wasm checks clean.

Findings:

- R1.1 (MINOR) scripts/perf-profile.sh - the traced artifact is ~800 MB
  for a 30 s run and lands wherever out_dir points (default
  perf-profile/<example> INSIDE the repo, which is not gitignored). A
  stray `git add -A` in the main checkout could stage it. Suggested:
  add `perf-profile/` to .gitignore.
  - Response: fixed in-round - perf-profile/ added to .gitignore.

- R1.2 (NIT) - `run_render_schedule` (25.5%) CONTAINS `render_system`
  (18.8%): parent/child spans both count toward the share denominator, so
  shares overlap rather than partition. Fine for RANKING (the stated
  purpose); T5 should not sum shares into a pie chart without a
  hierarchy-aware pass. Noted for T5; left as-is.

## Round 2 (2026-07-19, user-requested addendum)

- VERDICT: APPROVE (stands)

Scope: the `profiling` cargo profile + samply-branch rebuild, added at the
user's request after field-testing (raw-address flamegraphs). Verified:
`.debug_info` grew 1.6 MB -> 929 MB in the profiling binary, our functions
carry `push %rbp` prologues (RUSTFLAGS applied), the samply build excludes
the trace feature (no span distortion in sampled costs), package opt-levels
mirrored explicitly (`inherits` does not carry package overrides - checked
against the built artifacts' behavior, not assumed), and the full SAMPLY=1
pass exits 0 with the artifact. Deferred-symbolication semantics (names
resolve at `samply load` from the on-disk binary) documented in the wiki.
