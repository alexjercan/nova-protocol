# Review: unified nova_probe run-harness spike (SPIKE.md)

- ROUND: 1
- SCOPE: the spike DOC and its seeded tasks (design review, not code review)
- REVIEWER STANCE: adversarial; the doc's author ran this review, so every
  claim was re-derived against the codebase / Bevy APIs rather than recalled.
- VERDICT: REQUEST_CHANGES (direction is sound; the doc has two factual
  errors, two unconsidered alternatives, and a sequencing hazard that will
  cause golden churn if built as ordered)

## MAJOR

- [x] M1. **The "top-N costliest systems from Bevy diagnostics" claim is
  factually wrong.** Bevy's `SystemInformationDiagnosticsPlugin` reports OS
  process CPU%/memory, not per-system execution times. There is NO built-in
  diagnostic for per-system timings; the sanctioned source is per-system
  tracing SPANS (the `trace` feature), viewed in Tracy or emitted by
  `trace_chrome`. Nothing in this repo wires any trace feature today
  (verified: no `trace_chrome`/`trace_tracy`/`"trace"` in any Cargo.toml).
  Fix: derive the top-N table by POST-PROCESSING the chrome-trace JSON
  (aggregate span durations per system) - one capture, two products (the
  table + the Perfetto attachment). Update the spike's Profiling section and
  T4's goal text.
  Response: fixed in the spike revision - top-N is now derived by post-processing the chrome-trace span JSON (Profiling section + report spec section 5 + T4 retitled and re-bodied). Ticked after re-reading the revised text.

- [x] M2. **One run cannot honestly produce both the FPS numbers and the
  profile.** Enabling `bevy/trace` + chrome-trace serialization (and, less
  so, samply sampling) adds per-span overhead that contaminates the frame
  times the same report presents as the scene's FPS. The spike implies one
  run yields all artifacts. Fix: the runner does TWO passes - pass 1 clean
  (FPS + timeline, no tracing compiled in), pass 2 profiled (chrome trace +
  samply, FPS ignored) - and the report labels which pass fed which section.
  Costs 2x runtime; correctness of the headline numbers is the tool's whole
  point. Update Architecture + T6.
  Response: fixed - two-pass runner (pass 1 clean FPS/timeline, pass 2 profiled) written into the spike's Profiling + Architecture sections and T6/T4 bodies; the report labels which pass fed which section.

- [x] M3. **Golden-timeline risk is understated, and the build order
  guarantees churn.** Three compounding problems the doc parks as one "open
  question":
  (a) *Host instability*: the repo's own CI-red bug (20260718-235837) showed
  llvmpipe fits ~6.4 sim seconds into an 18 s wall window - frame counts,
  event interleaving across independent chains, and variable trajectories
  differ structurally between a dev-GPU run and CI. A single total-order
  golden may never match across hosts; comparison likely needs per-track
  partial order with value tolerances, which is a harder design than the doc
  admits.
  (b) *Snapshot fatigue*: goldens that churn on every legitimate content
  tweak train the operator to bless reflexively, and a rubber-stamped golden
  detects nothing. The doc has no bless discipline (e.g. bless requires the
  diff in the commit, reviewer eyeballs it).
  (c) *Direct collision with the queued v0.8.0 content tasks*: 20260718-152313
  (campaign polish, Shakedown -> Broadside) and 20260716-174729 (Gauntlet
  timer) will change the very scenarios examples like `19_broadside` drive -
  any golden committed before those land churns immediately.
  Fix: re-cut T3 - the recorder (T2) lands first and its timelines are
  rendered in the report WITHOUT goldens; goldens come after (i) T2's
  empirical stability data exists and (ii) the campaign-polish tasks land, or
  goldens are scoped to the stable section examples (01-07) only. T3 also
  defines the bless discipline.
  Response: adjudicated by user 2026-07-19 - goldens DEFERRED to backlog (20260719-112245 retagged backlog p0 with the deferral rationale + entry gate); invariant assertions adopted instead (new task 20260719-114931, p72); the report renders the timeline without a golden diff and reserves the layout spot.

- [x] M4. **The correctness divergence skipped two credible alternatives**,
  so the "golden timeline" choice was not fully weighed:
  (a) *Deterministic replay* (seeded RNG + fixed timestep + recorded inputs):
  the standard way to make timelines exactly comparable. Probably infeasible
  here (avian physics + f32 accumulation + variable render rate), but the
  doc must say WHY it was rejected, because if it were feasible it would
  obsolete tolerance-based drift entirely.
  (b) *Continuous invariant assertions* (health never negative, speed cap
  respected, scenario acts monotonic): catch bugs goldens cannot (goldens
  only detect change-vs-last-bless, not always-been-wrong), are immune to
  timing noise, and complement rather than replace the golden. Cheap to add
  to the recorder. Add both to Options considered; recommend invariants as a
  T2/T3 complement.
  Response: both alternatives added to Options considered - deterministic replay documented as rejected (avian physics + f32 accumulation + variable render rate; user concurred it is too hard to get right with avian), invariant assertions adopted as the chosen mechanism.

- [x] M5. **Process hazards in the seeded backlog.**
  (a) *Master still shows 20260718-152230 as OPEN with unticked steps*
  (verified) - the absorb note lives only on this branch, so a parallel
  session working master can legitimately pick it and REDO the report
  generator. Land the branch (or a master-side stub note) promptly.
  (b) *Priority inversion*: T5 (p55) outranks its own dependencies T2 (p54),
  T3 (p52), T4 (p50), and tatr has no machine-readable dependency field -
  a priority-order picker grabs T5 first and stalls. Re-slot strictly
  descending along the dependency chain (e.g. T1 56, T2 55, T3 53, T4 52,
  T5 50, T6 48) so naive priority order IS dependency order.
  Response: (a) branch squash-landed to master this cycle, so master now shows 152230 as absorbed; (b) priorities re-slotted strictly descending along the dependency chain (T1 76, T2 74, invariants 72, T4 70, T5 68, T6 66) and the family leads the v0.8.0 queue per user direction.

## MINOR

- [x] m1. **Rename-first is cosmetic churn.** T1 (rename) touches scripts,
  Trunk wiring, the example, workspace members - purely mechanically -
  before any new value is proven, and T2 does not actually need the new
  name, only a module to live in (nova_perf already provides one). Consider
  building T2 inside `nova_perf` and folding the rename into a later task
  once the tool's final shape is known; at minimum, T1's "foundation"
  framing should be weakened to "can happen any time".
  Response: user override 2026-07-19 - the rename to nova_probe stays the FIRST task (user prefers starting with the refactor); the finding is recorded but not applied.

- [x] m2. **The capture schema lacks run metadata and the doc never makes
  extending it an explicit work item.** Renderer/GPU, resolution, preset,
  git SHA, host class exist nowhere in the JSON/CSV (renderer is currently
  inferred from the results dir NAME). Baseline deltas and the report's Run
  summary both need them, and per-renderer thresholds (m4) depend on them.
  Name it as a step (fits T1 or T5).
  Response: fixed - run-metadata schema extension named explicitly in the spike Architecture and T1's Goal.

- [x] m3. **"One self-contained HTML" contradicts the attachments.** The
  chrome-trace JSON and samply profile are sidecar files, so the real
  artifact is a run DIRECTORY (self-contained report.html + attachments).
  Spec it that way, and add a machine-readable `checks.json` sidecar so an
  agent consumes verdicts without parsing HTML.
  Response: fixed - artifact spec'd as a run DIRECTORY (self-contained report.html + sidecars) plus a machine-readable checks.json, in the report spec and T5.

- [x] m4. **The 16.6 ms budget check must be per-renderer-class.** The
  v0.7.0 baseline has sw at 86-126 ms and web at 34-39 ms - a flat budget
  check would permanently WARN/FAIL those platforms, training reviewers to
  ignore the banner. Budget/threshold per renderer class, or informational
  outside native-GPU.
  Response: fixed - FPS thresholds per renderer class; budget check informational outside native-GPU (report spec check list + T5 notes).

- [x] m5. **The "no error!/panic in log" check needs an allowlist.** The
  10_playable diagnosis noted benign "damage-0.00 impact spam"; a naive
  scan would flag it. Start with a per-example allowlist and a note that a
  growing allowlist is itself a smell.
  Response: fixed - per-example allowlist for known-benign spam in the log-scan check, with the note that a growing allowlist is itself a smell.

- [x] m6. **v0.8.0 scope creep is real and undeclared.** Six substantial
  tasks were added to a release whose theme is docs + tooling consolidation,
  roughly doubling the tooling strand. Options: keep all six in v0.8.0,
  or land T1/T2 (+report growth) in v0.8.0 and push T3/T4/T6 to backlog for
  v0.9.0. User's call; the spike should record the decision either way.
  Response: adjudicated by user 2026-07-19 - all six tasks stay in v0.8.0 (the tooling feeds the release's docs + consolidation theme); only the golden task moves to backlog. Recorded in the spike's Recommendation.

## What survives the review (explicitly)

- The unified run-harness direction itself: one tool, one report, correctness
  + FPS + profile over the existing autopilot examples. No finding above
  attacks the goal; they attack the mechanism details and ordering.
- Grow-nova_perf (vs a new crate), auto-checks + human/agent final verdict,
  chrome-trace + samply over Tracy for headless automation, criterion bench
  staying separate, examples_smoke staying the fast gate - all re-derived and
  reaffirmed.
- The report content spec's seven sections stand, subject to m3/m4.

## Round 2 (2026-07-19, close-out)

User adjudicated round 1 directly: invariants over goldens (goldens to
backlog), two-pass runner accepted, chrome-trace-derived top-N accepted,
rename stays first (m1 overridden), all six tasks stay v0.8.0 (m6), family
priorities lead the sprint. All MAJORs and applicable MINORs applied to
SPIKE.md and the seeded task bodies in the same commit as this round; each
fix re-read in the revised text before ticking.

VERDICT: APPROVE
