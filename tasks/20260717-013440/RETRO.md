# Retro: cubemap_alt.png.meta not in meta_check Paths

- TASK: 20260717-013440
- BRANCH: meta-check-cubemap-alt (landed on master as 7af34490)
- REVIEW ROUNDS: 2 (R1: REQUEST_CHANGES on narrative-layer findings; R2: APPROVE)

## What went well

- Investigation before fix falsified the task's premise (the sky never
  rendered flat - the bcs fallback reinterpret runs the same frame the image
  lands) AND exposed the real trap: the "obvious" one-line meta_check fix
  alone would have shipped an invisible sky, because the bcs observer couples
  the reinterpret and the Cube-view set in one single-layer-only branch.
  Reading the 40-line dependency source was the highest-value step of the
  task; no test could have caught it (every skybox test was headless).
- The out-of-context review pass earned its keep again: it refuted two
  engine-semantics claims I had written from reasoning rather than source -
  the missing-view failure mode (bevy's skybox sanity check warn_once's and
  skips; it does not crash) and the "get_mut emits Modified" premise (the
  AssetMut guard tracks writes, not borrows). The second refutation killed a
  bogus work item in follow-up task 20260717-111558 before anyone spent a
  cycle on it.
- Fail-first discipline paid three times: the app-config test recorded the
  1-vs-6 loader evidence; the post-commit sabotage (`if false &&`) proved both
  new view assertions can fail; and the no-churn pin PASSING under that same
  sabotage confirmed it pins the write-guard, not the fix - so its role was
  understood, not assumed.
- Example 19 as the verification vehicle: one run covered the menu backdrop
  swap, the scenario load, the teardown+reload (Retry) window the fix is
  about, a victory screenshot for the eyeball, and the absence of the
  warn_once canary - five pieces of evidence for one build.

## What went wrong

- R1.1: I wrote "fatal wgpu validation error" for the missing-view case into
  FIVE sites (two doc comments, a test doc, both task files) from a plausible
  mental model of wgpu bind-group validation. Root cause: I applied
  verify-engine-guarantees-in-source to the dependency I was FIXING (bcs) but
  not to the consumer whose failure mode I was CLAIMING (bevy_core_pipeline's
  skybox, which has an explicit sanity check that degrades gracefully). A
  failure-mode claim names concrete behavior of concrete code; it needs the
  same source-read as the mechanism itself.
- R1.2, same root cause one crate over: "Assets::get_mut emits Modified" is
  false in bevy_asset 0.19 (AssetMut queues Modified only on actual mutable
  deref), and that unread claim seeded a wrong work item in the follow-up
  task's goal.
- piped-cargo-masks-exit-code recurred (third occurrence): the first
  fail-first evidence run ended with `| tail; echo EXIT: ${PIPESTATUS[0]}`,
  so the harness saw exit 0 on a COMPILE ERROR and reported the run
  "completed". The lesson was already on the ledger at x2 and I still
  reflexively built a masking pipeline; only reading the output text caught
  it.

## What to improve next time

- Any failure-mode claim written into a doc/comment ("X panics", "Y is a
  validation error", "Z re-uploads") is an engine-semantics hypothesis: grep
  the consumer for the actual error/validation/warn site and cite file:line
  in the claim, exactly as for ordering/observer guarantees.
- Cargo evidence runs: never end the command with echo/tail that eats the
  exit code - run the cargo invocation bare (backgrounded) and read its
  output text; if a pipe is unavoidable, `set -o pipefail` first.

## Action items

- [x] Follow-up tatr task 20260717-111558 (mod-shipped cubemaps ride the
  fallback path; upstream bcs view fix) - filed during /work, corrected
  during review (bogus churn item removed with the refutation recorded).
- [x] Ledger: bump `verify-engine-guarantees-in-source` (x5, failure-mode
  variant), `out-of-context-review-pass` (x28), `piped-cargo-masks-exit-code`
  (x3 -> Pending promotions).
- [ ] Pre-existing nit found during the verify run: examples/19_broadside.rs:121
  `let mut advance` needs no `mut` (unused_mut warning under --features
  debug); fixed in a separate trivial commit on master alongside this retro.
