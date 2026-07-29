# REVIEW - 20260729-002952 (probe FPS regression investigation)

- ROUND: 1
- VERDICT: APPROVE
- REVIEWER: in-context (see caveat)

## Caveat on the reviewer

Flow's default is an out-of-context reviewer. This session was directed not to
spawn subagents, so round 1 was done in-context by re-deriving every claim from
the artifacts rather than re-reading the prose. That is weaker than an
independent read and is recorded here honestly. The diff is documentation and
one new backlog task - no code, no behavior change - so the risk surface is the
correctness of the CLAIMS, which is what the re-derivation targets.

## What was reviewed

Docs-only diff: `tasks/20260729-002952/NOTES.md` (new), that task's `TASK.md`
(closed + outcome), and `tasks/20260729-205957/` (new backlog task). No source
files touched, so no build/test surface.

## Claim-by-claim verification

| Claim | How checked | Result |
|---|---|---|
| Per-example `mean_ms`/`p95_ms` table | parsed all three `frametime.csv` sets directly | matches |
| `scenario` +0.05%, `playable` -0.06%, `perf_baseline` -8.3%, `lifeline` -18.3% | recomputed from the means | matches |
| All `fps_within_baseline` PASS with negative delta at `a6d06220` | read `checks.json` for each example | matches (`-2.6 / -5.5 / -12.9 / -51.6`) |
| `NovaOsTtcFontLoader` absent from post-fix traces | scanned `trace.json` names for `Font`/`Ttc` in all three sets | absent at `82bc2dc9` and `a6d06220` |
| The loader no longer exists in the tree | `grep -ril ttc crates/ assets/ examples/` | zero hits |
| `c3ee1988` is the removing commit | `git log -S ttc -- crates/` and `git log -- assets/fonts/*` | both name it |
| Comparable run environment | `quality`/`resolution`/`backend` columns across all nine CSVs | identical (`default` / `1280x720` / `vulkan`) |
| Operator mods never leaked into runs | `grep -ricE 'gauntlet\|the-ledger' probe-runs/*/*/run.log` | 0 in every run |

## Findings

### 1. (MAJOR, fixed) Wrong commit count

NOTES claimed "36 further commits of HUD/UI work" between `82bc2dc9` and
`a6d06220`. `git rev-list --count 82bc2dc9..a6d06220` is **15**. Corrected. The
number is load-bearing for the "it stayed fixed across continued development"
argument, so an inflated count overstates the evidence.

### 2. (MINOR, accepted) The host-noise argument is inference, not proof

The claim that `82bc2dc9`'s residual +5-6% was host noise rests on that
session's `lifeline` p95 of 112 ms against ~24-25 ms everywhere else. That is
strong circumstantial evidence and the run was not repeated. NOTES states it as
an inference with its supporting number rather than as measurement, which is the
right altitude. Accepted as written; a re-run would not change the closing
verdict, since HEAD is already at baseline.

### 3. (MINOR, accepted) Trace-total deltas are not normalized

The render-side section reports raw `RenderApp` / `RenderExtractApp` totals from
the traced pass, which covers differing frame counts and includes new
instrumentation spans. NOTES says so explicitly and defers to the timed
`frametime.csv` pass for the verdict. Correct handling - the alternative
(normalizing per frame) would add precision the conclusion does not need.

### 4. (NIT, recorded) Mixed `git_sha` inside `probe-runs/82bc2dc9/`

Two of that folder's CSVs record the parent SHA `1a0c11b5`. NOTES records this
as an artifact quirk without acting on it, which respects the task's own "do not
hand-edit historical probe artifacts" constraint. Both SHAs are after
`c3ee1988`, so the conclusion is unaffected.

## Definition of Done

1. notes - NOTES.md holds the regression summary and the commit attribution from
   JSON/CSV/trace files. **MET**
2. cmd - probe commands and the resulting `checks.json` / `frametime.csv` deltas
   are recorded. **MET**
3. manual - the report states 20260729-000956 resolved the font-loading part in
   full, and that no residual render-side regression warrants an optimization
   task. **MET**
4. manual - the report states 20260729-003352 was available and is what kept the
   run folders comparable. **MET**

## Verdict

APPROVE. One real error found and fixed; the remaining findings are accepted
framing, not defects. The conclusion - regression fixed by `c3ee1988`, nothing
left open - is supported by the artifacts.
