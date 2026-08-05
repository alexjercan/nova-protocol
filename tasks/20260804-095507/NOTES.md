# Notes: Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

Goal in one line: exercise the rebuilt fleet as a WHOLE - the only place that
happens - and read the resulting report.

## Owner decisions (2026-08-05) - these override anything below

| Question | Decision |
|-|-|
| The CI smoke gate | ALREADY RUN by the owner, green. Not a Step. |
| CI budget / `timeout-minutes: 60` | Not investigated here. If CI blows the budget, the owner finds out on push. |
| Where to run | `master`, in place. No sprout worktree. |
| Where the report goes | probe's DEFAULT dir, `probe-runs/` (gitignored). NOT committed. |
| Baseline comparison | DROPPED. The retained v0.7.0 numbers are other scenarios; there is nothing to compare against. |
| The command | one plain probe run, default output. No `--baseline`, no custom `--out`. |

What survives: run the fleet, read the verdict, act on what it surfaces. The
deliverable is a green run and whatever it forces, not a stored artifact.

## What changes

Before: each of the six preceding tasks proves its own category with
`probe run <category>`. Nothing runs all of them in one invocation, so
cross-category effects (the run policy misfiring, the aggregate report, a
category that only fails under `--all`'s sequencing) are unproven.

After: `probe run --all --fps` runs green as one invocation, writing its
report.html + checks.json to the default `probe-runs/` (gitignored). The
report is read, not stored.

This is not new production code. It is a verification pass, plus whatever
fixing the full-fleet run turns out to demand.

## Surfaces

| File | Why |
|-|-|
| (none, by intent) | The deliverable is a green run. Code edits here are unplanned fixes to whatever the full-fleet run surfaces. |
| `crates/nova_probe/src/bin/probe/native/spec.rs` | `resolve_spec` with `all: true` walks the whole catalog minus `NOT_PROBED`. Verifying `screenshots/` is excluded means verifying THIS, not just reading the contract. |
| `crates/nova_probe/src/run_report/` | `manifest.rs`, `checks.rs`, `html.rs` produce the artifacts being retained. |
| `tasks/20260802-115955/TASK.md` | The epic Done Means this discharges (`probe run --all --fps`, and the `playing_since` absence grep). |

## Data and interfaces

None added. The run writes, to the default `probe-runs/` (gitignored):

```
index.json       the aggregate agent surface: does everything still work
probe-all.json   the gate; exit code mirrors the WORST row
report.html      per-example human-readable report
checks.json      OK / WARN / FAIL per example per pass
frametime.csv    per-run, stress/ only
```

The DoD proofs:

```bash
nix develop --command cargo run -p nova_probe -- run --all --fps
! rg -n "run ended with the scripted run unfinished|playing_since" examples
# test: catalog_matches_disk
```

Read `index.json` / `checks.json`, not the HTML. Verdict is read TOGETHER with
`measured` ("n/total"), never alone.

## Shape

```
093855 (contract + policy)
   |         |         |         |
   v         v         v         v
093934    093950    094021    (093950)
(systems)  (sections) (ui)        |
   |         |         |          v
   v         |         |       094006 (stress)
093910       |         |          |
(retire)     |         |          |
   +---------+---------+----------+
                 |
                 v
          095507  THIS TASK
          probe run --all --fps
                 |
                 +--> probe-runs/ (gitignored, read not kept)
                 +--> frame-time numbers written into this task's Notes
                 +--> anything only --all surfaces -> fix or file
```

## The title's premise was wrong

"as CI will" implied `probe run --all --fps` is what CI runs. It is not. No
workflow invokes probe at all. CI's fleet gate is
`xvfb-run --auto-servernum cargo test -p nova-protocol --test examples_smoke
--features debug` (`.github/workflows/ci.yaml:108`), inside a job with
`timeout-minutes: 60` (:24) covering a possible cold build.

So this task has TWO runs, and conflating them would have left the real gate
unverified while producing a report nothing enforces:

| | Gate | Evidence |
|-|-|-|
| What | `cargo test --test examples_smoke` | `probe run --all --fps` |
| Who | CI, every PR | the owner, once, here |
| Proves | reaches Playing, no panic, no command errors | correctness + frame time |
| Fails the build | yes | no |

The distinction still holds, but the gate column is already discharged: the
owner ran the smoke command and it passed. It is not a Step.

The BUDGET question the sprint creates - the smoke run is sequential
(`examples_smoke.rs:250`, one `cargo run` per example) and the new `stress/`
runs exist specifically to be heavy, so net TIME rises against
`timeout-minutes: 60` - is DEFERRED by owner call. CI reports it on push; no
local estimate is worth the time here.

## Consequences and open questions

- Depends on all six others. It is the last task in the chain and cannot start
  early - which also makes it the one most likely to be squeezed at the end of
  the sprint. Its value is precisely in not being skipped.
- Restored from `20260802-120029` Step 9 and its `playing_since` grep, which
  were dropped when that task closed SUPERSEDED and had no owner at all until
  the spike's review caught it.
- OWNER-DECIDED - where the report lives: probe's default `probe-runs/`,
  gitignored, not committed. The earlier plan to commit it under
  `tasks/20260804-095507/probe-results/` (on the `tasks/20260716-123551/`
  precedent) is DROPPED. Consequence, stated plainly: this run leaves no
  artifact behind in the repo, so the record of it is this task's Notes and
  the retro - the numbers must be written down there or they are gone.
- OWNER-DECIDED - baseline comparison is DROPPED. The retained v0.7.0
  results cover other scenarios; with `broadside` retired and the `stress/`
  runs new, there is no series that both sides share. Nothing to compare, so
  the task does not pretend to.
- Runtime, estimated rather than guessed: `stress/` uses the full 180 + 900
  frame window (`capture.rs:94,97`) and `env.rs` sizes each fps pass's deadline
  at `1080 / FPS_FLOOR(2.0) + 45s` = 585s. That is the worst-case BOUND, not
  the expected time - at a realistic llvmpipe rate the capture is a couple of
  minutes - but four `stress/` fps passes plus correctness passes over ~21
  other examples is a long single command either way. If it proves unwieldy,
  `--all` and `stress --fps` as two invocations still discharge the epic's
  Done Means; the single-invocation shape is not itself the requirement.
- Scope discipline, now in the Steps as FILE-not-FIX: this is the sprint's last
  task and the natural place for leftovers to accumulate. Anything beyond a
  one-line correction gets a task, not a commit here. The CI-budget question
  above is the most likely candidate.
