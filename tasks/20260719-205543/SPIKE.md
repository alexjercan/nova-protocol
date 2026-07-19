# SPIKE: probe-all - every example evaluable, one aggregated report

- TASK: 20260719-205543
- DATE: 2026-07-19
- QUESTION: can most/all examples become profilable + fps-able +
  correctness-harnessed under probe, what does a multi-run CLI with an
  aggregated status report look like, and where are the honesty traps?

## Findings: what each capability actually costs

| Capability | Mechanism | Has it today | Cost to extend |
|---|---|---|---|
| profilable | `--profile`/`--samply` trace pass (bevy per-system spans + chrome writer) | ALL 21 - already universal | zero: the traced pass is example-agnostic by construction (separate build, TRACE_CHROME env); nothing to wire per example |
| correctness (timeline + invariants) | `app.add_plugins(nova_probe::nova_timeline())` + `nova_probe::nova_invariants()` - inert without probe's env | scenario, playable only | 2 lines per example x 16 unwired autopilot examples. Generic payoff WITHOUT any per-example work: `run_completed`, `reached_playing` and `invariants_held` (engine bounds: health, velocity finiteness, variable finiteness, entity leak) all become MEASURED - every example that today reports OK-with-coverage 2/6 jumps to 5/6 |
| fps-able | `nova_frametime()` - inert without env | perf_baseline only | 1 line per example. See the honesty section: capture everywhere is fine, BASELINES stay a perf/ concern |
| markers / monotonic depth | `probe_marker` beats + `.monotonic([...])` | playable (7 beats), scenario (2 monotonics) | per-example judgment; bounded by the standing rule: only variables the scenario DESIGN promises one-way |

Current probe surface fits multi-run without rework: `RunOptions.example`
is one String and the run dir is derived as `probe-runs/<example>/` -
a multi-run is a loop producing today's per-example artifacts unchanged,
plus an index above them. `run()` needs no signature change.

The examples reorg (20260719-193728) is what makes "all" well-defined:
`autoexamples = false` + the `[[example]]` catalog is a machine-readable,
drift-pinned list, and category dirs give natural selection units.

## Design

### CLI (extends `probe run`, no new verbs)

```
probe run playable                      # today, unchanged
probe run playable,scenario             # comma list
probe run gameplay                      # category name -> its examples
probe run --all                         # every cataloged example minus NOT_PROBED
```

- Spec resolution: exact example name wins; else a category dir name
  expands to its cataloged members; else error listing both. (No
  ambiguity today: no example is named like a category; the catalog
  parser can assert this stays true.)
- Catalog discovery: parse the root Cargo.toml `[[example]]` blocks.
  The parser LIVES IN nova_probe and `tests/examples_smoke.rs`'s
  `catalog_matches_disk` switches to calling it (root package already
  dev-depends on nova_probe) - one parser, pinned by the existing drift
  test, no cargo_metadata dependency.
- RECOMMENDATION (adjudicate): bare `probe run` ERRORS with the runnable
  list instead of running all - an accidental 30-minute sweep is a worse
  default than one more flag. The user's message allowed either.
- Multi-mode flag rules (honest combinations, same spirit as today):
  `--profile`/`--samply`/`--fps`/`--release` apply to EACH example;
  the sweep matrix flags (`--scenario`/`--preset`) and `--platform web`
  are single-example concerns and are REJECTED with a list/category/all
  spec.

### Execution

Sequential on purpose (cargo target lock; one Xvfb reused across runs;
no cross-run frame contamination) - same reasoning as the smoke suite.
Per-example timeout stays (180s default); a timed-out or failed example
marks its ROW and the sweep CONTINUES - one hung example must not eat
the other twenty. Wall-clock, warm target: ~1-2 min/example (incremental
link + ~40s autopilot lifetime) -> categories are the everyday unit
(3-7 examples, single-digit minutes); `--all` (~19 runnable, 25-40 min)
is the pre-release / nightly / "evaluate everything" sweep.

### Aggregated report

- `probe-runs/index.html` + `index.json` + `probe-all.json` (the
  aggregate manifest: spec, per-example outcome + duration, started/git
  identity - the analog of probe-run.json, and the gate for
  `probe report probe-runs/` re-rendering the index).
- One row per example: category, VERDICT badge, `measured n/total`,
  six per-check glyphs (PASS/WARN/FAIL/SKIP), duration, link to the
  example's own report.html. Header: totals (OK/WARN/FAIL/NO_DATA),
  aggregate coverage, and the overall verdict = WORST row; process exit
  mirrors it (any FAIL/NO_DATA -> nonzero).
- Exclusions are IN the report: a NOT_PROBED section listing each
  excluded example WITH its reason (the no-silent-caps rule). Initial
  list: `render_scale_shot` (a BCS_SHOT real-GPU pixel capture with no
  self-ending autopilot - under probe's Xvfb it would time out AND its
  point, correct pixels, is exactly what a software framebuffer cannot
  judge). `perf_baseline` IS probed (runs fine headless; its fps row is
  its purpose).
- index.json is the agent surface: verdicts + measured per row, so a
  session can read one file to answer "does every feature still work".

### The fps honesty resolution

Earlier guidance said frame capture belongs on measurement scenes only.
The spike splits that concern in two:
- CAPTURE everywhere is harmless and useful (inert line; `--fps` then
  yields a stats row on any example - "did this feature change frame
  cost" becomes askable ad hoc).
- BASELINE GATING stays where numbers are trustworthy: the fps check
  already only fires when `--baseline` is given; scripted correctness
  runs make noisy numbers, dev-profile numbers are not baselines
  (standing lesson).
- GAP TO CLOSE with the wiring: `RunMeta` records backend/adapter/
  resolution/quality/sha/host but NOT the build profile - add
  `profile: dev|release` (cfg!(debug_assertions) at capture time) so
  every stats row names its profile and the report can label dev rows
  "not a baseline". Without this, fps-everywhere invites apples-to-
  oranges deltas.

## Adversarial pass (what could make this a lie or a regret)

1. **Aggregate hides SKIPPED**: a green table of OK badges where half
   the rows measured 2/6 would be the old dishonesty at a new layer.
   Countered: measured column is mandatory per row, coverage is in the
   header, overall verdict is worst-of. An unwired example is VISIBLY
   thinner than a wired one - which is also the standing motivation to
   finish the wiring sweep.
2. **Invariant false positives at scale**: 16 examples newly under
   engine-bound checks; thresholds were tuned on scenario/playable only.
   Countered: the wiring task's exit gate is one full `--all` run with
   every report READ, and any firing invariant either exposes a real bug
   (a finding, filed) or a wrong bound (tuned, documented). Wiring is
   not "add two lines and trust".
3. **Duplicates CI smoke**: it does not - the smoke suite is the fast
   pass/fail CI gate on stderr contracts; probe-all is a local/nightly
   EVIDENCE artifact (timelines, invariants, frame stats, reports).
   Different jobs; document the split; do NOT add probe-all to CI now
   (30 min of Bevy runs per push is not a gate, it is a queue).
4. **One hung example poisons the sweep**: per-row timeout -> FAIL row,
   sweep continues (matches the hardening rule: a dead run produces a
   FAILING report, not no report).
5. **Catalog parser drift**: two parsers (probe + drift test) would
   diverge; sharing one through nova_probe, pinned by the existing
   test, removes the second copy.
6. **Marker/monotonic overreach**: wiring depth beyond what a scenario
   design promises produces flaky invariants (the goldens lesson).
   T3 stays scoped to design-promised beats and is separately reviewable.

## Proposed task cuts (v0.8.0, in this order)

- **T1 - multi-run + aggregate (p59)**: spec parsing (list/category/
  --all), shared catalog parser in nova_probe (drift test switches to
  it), sequential runner with continue-on-failure, index.html +
  index.json + probe-all.json + report gate, NOT_PROBED with reasons,
  flag-combination rules. E2E: `probe run ui` (category) and a
  two-example list; the aggregate over partly-unwired examples SHOWS the
  thin rows honestly.
- **T2 - wiring sweep (p58)**: `nova_timeline()` + `nova_invariants()`
  into all 16 unwired autopilot examples, `nova_frametime()` alongside
  (inert), `RunMeta.profile` field + report label. Exit gate: a full
  `probe run --all`, every row read, every firing invariant adjudicated
  (bug filed or bound tuned). Close-out records the aggregate.
- **T3 - depth markers (p52, or backlog)**: design-promised
  `probe_marker` beats + monotonics for the examples whose in-example
  assertions already encode outcomes (sections: fired/hit; broadside:
  stage progression) so reports SHOW the feature working, not just the
  process surviving.

T1 before T2 on purpose: the aggregate makes the unwired state visible
(motivating and validating T2), and T2's exit gate needs T1's `--all`.

## Open adjudications for the user

1. Bare `probe run`: error-with-list (recommended) or run-all?
2. fps wiring breadth: everywhere (recommended, inert + profile-labeled)
   or perf/ + gameplay/ only?
3. T3 in-sprint at p52 or moved to backlog?
