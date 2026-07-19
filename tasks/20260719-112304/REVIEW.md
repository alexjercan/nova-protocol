# Review: unified run report (report.html + checks.json)

- TASK: 20260719-112304
- BRANCH: feature/probe-run-report

## Round 1

- VERDICT: APPROVE (one MINOR, addressed in-round)

Shared-session caveat: implementer and reviewer are one session; the
load-bearing claims were re-derived against the artifacts, not the diff:

- **Spec-vs-diff**: the spike's 7-section report spec walked section by
  section against the rendered output (real e2e report contains every
  section marker + the fixture test pins them); the five inherited review
  notes (per-name counts, rank-only profile, WARN-only FPS, truncation
  semantics, SKIPPED-never-held) each located in the code AND asserted in
  a test or displayed in the artifact.
- **Would-it-fail audit**: every check has both directions pinned -
  truncated timeline -> run_completed FAIL; planted violations ->
  invariants FAIL with "health_bounds x2" in the detail; +25% mean ->
  WARN while +5% passes AND a WARN alone never fails the verdict; planted
  panic -> log FAIL; all-artifacts-removed -> all SKIPPED. The one test
  bug found en route (a non-unique replace anchor doubling the planted
  violations to x4) was a RIG bug caught by its own literal assertion -
  the fix tightened the anchor, not the expectation.
- **Real-run evidence**: the e2e is a REAL armed 10_playable run dir, not
  the fixture - verdict OK, run_end at frame 1372, 0/1372 violation
  frames, 1349 pulses collapsed, real git SHA in the summary, 23 KB
  self-contained HTML (no script/external refs, pinned).
- **Honest e2e re-cut**: the plan's "copy in a frametime.csv +
  trace.json" clause was dropped WITH the reasoning recorded (planting
  foreign artifacts fabricates a run that never happened); full-artifact
  rendering is fixture-pinned instead. The step text was amended, not
  quietly ticked.
- **Exit-code discipline**: the background e2e reported all-green
  step exits EXCEPT wasm (101) - reading each exit individually caught
  the bin's missing wasm gate; fixed with a stub main and re-verified.
- **Absorbed task closed truthfully**: 20260718-152230's DoD items each
  dispositioned (what shipped where), not hand-waved.

Findings:

- [x] R1.1 (MINOR) crates/nova_probe/src/run_report.rs (log_clean) - the
  check scans for " ERROR " with spaces, but bevy's log format wraps the
  level in ANSI color codes when the run's stderr is a TTY, and some
  formats emit "ERROR" followed by a narrow space variant; a real error
  could slip the exact-substring net. The captured-by-script path (T6)
  pipes to a file (no TTY, no ANSI), so today's contract holds, but the
  check should strip ANSI escapes before scanning to be robust to how the
  log was captured.
  - Response: fixed in-round - log lines are ANSI-stripped before the
    scan, pinned by a test with a colored ERROR line.
