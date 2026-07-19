# Review: probe hardening (trustworthy verdicts)

- TASK: 20260719-174541
- BRANCH: fix/probe-hardening

## Round 1

- VERDICT: APPROVE

Shared-session caveat applies doubly here - the implementer is fixing
findings a fresh-eyes agent produced, so this round verifies the FIXES
against the FINDINGS one by one rather than re-deriving from scratch:

- **Finding 1 (stale artifacts)**: clean_out_dir removes exactly probe's
  own artifact filenames (enumerated, no recursive wipe of user paths)
  before every run; the manifest stamps identity so any surviving
  confusion is visible. Verified the RUN_ARTIFACTS list against every
  filename any pass writes - all nine covered plus the manifest itself.
- **Finding 2 (timeout aborts)**: RunOutcome replaces the Err path;
  e2e 3 is the live proof - a forced 8 s timeout produced EXIT=1 with a
  COMPLETE report (process_exit FAIL timed-out, run_completed FAIL
  truncated, reached_playing PASS at frame 25). The profiled-pass degrade
  and pre-run --baseline validation both read as implemented; the
  baseline validation reuses the real parser, so "exists but corrupt"
  also fails fast.
- **Finding 3 (dropped exit status)**: process_exit leads the check rows,
  fed only by the manifest (SKIPPED for foreign dirs - correct: the
  report cannot know what it did not supervise). Pinned by the manifest
  round-trip test and e2e 1/2/3.
- **Finding 4 (all-SKIPPED OK + misdirection)**: NO_DATA + nonzero exit
  for zero-measured (pinned), measured n/total beside every verdict
  (banner, checks.json, stdout), armed-vs-unwired skip details (pinned +
  e2e 1 shows the exact new wording). The OK-with-coverage refinement is
  recorded in Steps with its reasoning - the harness assertions ARE
  evidence; the skill's agent rule now requires run_completed +
  invariants_held measured for gameplay claims.
- **MINORs**: teardown reset pinned with the reload rig (live regression
  still fires - the test asserts BOTH directions); fps improvement PASS
  pinned at -50%; entries cross-check pinned at 14-vs-10; whole-word
  ERROR pinned against line-initial ERROR and TERRORD; Xvfb liveness +
  band move read as implemented; label env pinned in clean_pass_env's
  test? - VERIFIED: the env test asserts NOVA_PERF_LABEL only via the
  fps=true path... see R1.1.
- **Docs sync**: skill/wiki/CHANGELOG updated in-branch; the skill's
  checks list matches the six shipped checks verbatim.

Findings:

- [x] R1.1 (MINOR) crates/nova_probe/src/bin/probe.rs (tests) - the
  clean_pass_env unit test was not extended for the new NOVA_PERF_LABEL
  behavior; the label derivation (out-dir file name) is unpinned, and a
  regression to the old "scene" default would silently break probe-vs-
  probe baselines again.
  - Response: fixed in-round - the env test asserts the label equals the
    out dir's name under --fps and is absent without it.
