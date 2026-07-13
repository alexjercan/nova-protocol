# REVIEW

- TASK: 20260710-143138 - CI: examples smoke test panics in taffy on GitHub
  runners only - diagnose and re-enable as blocking
- BRANCH: fix/ci-smoke-gate (1 commit on master: f1b21b5)

## Round 1

Scope reviewed: `git diff master...fix/ci-smoke-gate` - .github/workflows/ci.yaml
(gate flip + comment rewrite), CHANGELOG.md (Unreleased entry), TASK.md
(close, superseded steps, Record).

Checks performed:

- YAML parse: ci.yaml parses clean (pyyaml safe_load). Job structure intact,
  8 steps, step renamed to "Examples smoke test", `continue-on-error` absent
  from the whole file.
- Step-name dependencies: single job ("fmt / clippy / test", name unchanged -
  branch protection contexts key on job names, not step names), no step
  `id:`s, no `needs:`, and no other workflow (deploy-page.yaml, release.yaml)
  or script references the step name. The only remaining occurrences of
  "Examples smoke test (non-blocking)" are inside this task's Record, quoting
  the historical run - accurate as written.
- Stale "non-blocking" references: grep across the repo finds only historical
  records - the 0.4.1-era CHANGELOG line, closed-task REVIEW/RETRO/SPIKE files
  (mostly the unrelated review-severity sense), and the dated v0.5.2 plan
  snapshot (docs/plans/20260713-v0.5.2-plan.md:15,91) which describes the
  state at planning time. No living doc claims the gate is non-blocking.
- Comment rewrite: the old comment pointed at
  docs/2026-07-10-skybox-cubemap-upload-race.md, which no longer exists
  (docs restructure folded it into the task's NOTES.md); the new pointer to
  tasks/20260710-143138/NOTES.md ("The taffy panic") resolves. The rewrite
  fixes a dangling reference in passing.
- Evidence verification (run 29283727248): push event, headBranch master,
  headSha 57da8ef, created 2026-07-13T20:46Z, workflow CI, conclusion
  success. Step "Examples smoke test (non-blocking)": success. Crucially I
  did NOT stop at the step conclusion - for a `continue-on-error: true` step
  the jobs API can report success even when the command failed - and pulled
  the job log: `Running tests/examples_smoke.rs`, `running 1 test`,
  `test harnessed_examples_reach_playing_without_panic ... ok`,
  `test result: ok. 1 passed; 0 failed; ... finished in 188.23s`. The pass is
  genuine, not a continue-on-error mask.
- Full-suite coverage: at 57da8ef, tests/examples_smoke.rs iterates all 12
  HARNESSED_EXAMPLES sequentially inside that one test function (per-example
  output is libtest-captured on pass, hence the quiet log), so "1 passed"
  covers the entire reworked suite including the in-example assertions and
  the command-error gate. The rework commit 4cbf94c is an ancestor of
  57da8ef, and 4cbf94c is NOT in the previous master run's sha (fbb6dda2) -
  so this run is indeed "the first master push after the rework", exactly as
  the Record claims. origin/master == 57da8ef, so it is also the only
  post-rework run to date.
- Determinism claim: verified from task history - the original TASK.md
  (created alongside 262e017, which made the step non-blocking) states
  "03_scenario deterministically panics on GitHub's ubuntu-latest runners"
  and that the panic "still reproduced on CI with the fix in" (i.e. at least
  two independent CI observations, against 15+ local non-reproductions).
- Record honesty: it says "worked around by replacement rather than
  root-caused", explicitly lists what was never learned (the corruption
  mechanism; the zen3-JIT theory neither confirmed nor refuted), states the
  evidence as exactly one green run of the full suite, and gives actionable
  reopening instructions (blocking step fails loudly with RUST_BACKTRACE=full,
  NOTES.md "The taffy panic" as the starting point, the
  LP_NATIVE_VECTOR_WIDTH / SwiftShader / qemu EPYC ladder still queued). The
  three investigation steps are marked SUPERSEDED `[~]`, not done - correct.
- `cargo fmt --check`: clean (no Rust touched on the branch, as expected).
- Not run per standing instruction: local cargo test / clippy (CI covers
  them; nothing on this branch compiles differently anyway).

### VERDICT: APPROVE

Findings:

- [x] R1.1 (MINOR) tasks/20260710-143138/TASK.md:69-73 - the closure rests on
  a single post-rework green run (n=1). Against the recorded failure mode -
  deterministic per-run on CI - one full green run on the real runner
  environment is sufficient to establish "the deterministic panic no longer
  fires", and I verified that run is genuinely green at the log level. What
  n=1 cannot rule out is the panic having become intermittent. I judge the
  close acceptable anyway: the step is now blocking, so the gate itself is
  the ongoing detector, and a wrong close costs exactly one loud red master
  run with a full backtrace and a documented investigation ladder - the
  outcome the task wants. Flagged so the thinness is on the record, not to
  block.
  - Response: accepted as recorded - the n=1 basis stays in the Record
    verbatim; the blocking step is the detector for the intermittent case.
- [x] R1.2 (MINOR) tasks/20260710-143138/TASK.md:70-72 - the Record cites the
  step conclusion ('step "Examples smoke test (non-blocking)": success') as
  its evidence. For a continue-on-error step that citation is weaker than it
  looks: the jobs API reports such a step as success even when the command
  fails, so a future reader re-verifying the close from the cited artifact
  could be misled. The underlying log is genuinely green (test result: ok,
  1 passed, 188.23s - verified this review), so the conclusion stands;
  suggest amending the Record to cite the log line ("test result: ok. 1
  passed" in job 86930991494) rather than, or alongside, the maskable step
  conclusion.
  - Response: fixed - the Record now cites the job log's test-result
    line and notes the step conclusion is maskable under
    continue-on-error.

No BLOCKER/MAJOR findings. TASK.md's opening "is currently
`continue-on-error: true`" (line 7-8) and the dated v0.5.2 plan's
"non-blocking" references were considered and judged historical-record
wording, not findings.

## Round 1 response verification (2026-07-14)

Responses landed in 86e7b1f; verified against the commit diff:

- R1.1: accepted as recorded - the Record's n=1 wording is untouched, as
  agreed. Ticked.
- R1.2: TASK.md's Record now cites the job log's `test result: ok. 1
  passed; 0 failed ... finished in 188.23s` line for
  tests/examples_smoke.rs (matching the log pulled during this review),
  notes the step conclusion is maskable under continue-on-error, and
  records that the single test iterates all twelve HARNESSED_EXAMPLES.
  The run id (29283727248) remains as the locator; the run has a single
  job, so the log is findable from it. Ticked.

VERDICT: APPROVE stands.
