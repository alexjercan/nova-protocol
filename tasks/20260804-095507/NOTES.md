# Notes: Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

Goal in one line: exercise the rebuilt fleet as a WHOLE - the only place that
happens - and keep the resulting report as the sprint's evidence.

## What changes

Before: each of the six preceding tasks proves its own category with
`probe run <category>`. Nothing runs all of them in one invocation, so
cross-category effects (the run policy misfiring, the aggregate report, a
category that only fails under `--all`'s sequencing) are unproven.

After: `probe run --all --fps` runs green as one invocation, its report.html +
checks.json are retained as the sprint's evidence, and the frame-time numbers
are recorded against the previous baseline.

This is not new production code. It is a verification pass whose OUTPUT is a
record, plus whatever fixing the full-fleet run turns out to demand.

## Surfaces

| File | Why |
|-|-|
| (none, by intent) | The deliverable is a run and a record. Code edits here are unplanned fixes to whatever the full-fleet run surfaces. |
| `crates/nova_probe/src/bin/probe/native/spec.rs` | `resolve_spec` with `all: true` walks the whole catalog minus `NOT_PROBED`. Verifying `screenshots/` is excluded means verifying THIS, not just reading the contract. |
| `crates/nova_probe/src/run_report/` | `manifest.rs`, `checks.rs`, `html.rs` produce the artifacts being retained. |
| `tasks/20260802-115955/TASK.md` | The epic Done Means this discharges (`probe run --all --fps`, and the `playing_since` absence grep). |

## Data and interfaces

None added. The evidence is:

```
report.html      the human-readable aggregate
checks.json      OK / WARN / FAIL per example per pass
frametime.csv    per-run, stress/ only
```

The three DoD proofs:

```bash
nix develop --command cargo run -p nova_probe -- run --all --fps
! rg -n "run ended with the scripted run unfinished|playing_since" examples
# test: catalog_matches_disk
```

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
                 +--> report.html + checks.json  = sprint evidence
                 +--> frame-time delta vs previous baseline
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

Gate first: a red smoke makes the probe report meaningless, and it is far
cheaper to run.

The gate also has a BUDGET consequence the sprint creates. The smoke run is
sequential (`examples_smoke.rs:250`, one `cargo run` per example, deliberately
not parallel). Today ~22 examples smoke. After the rebuild: three retire, but
`widget_zoo` joins (owner call on `20260804-094021`) and three new `stress/`
runs appear that exist specifically to be heavy. Net count barely moves; net
TIME rises. Whether that still fits 60 minutes cold is a question this task
must actually answer, not assume.

## Consequences and open questions

- Depends on all six others. It is the last task in the chain and cannot start
  early - which also makes it the one most likely to be squeezed at the end of
  the sprint. Its value is precisely in not being skipped.
- Restored from `20260802-120029` Step 9 and its `playing_since` grep, which
  were dropped when that task closed SUPERSEDED and had no owner at all until
  the spike's review caught it.
- RESOLVED - where the evidence lives: committed under
  `tasks/20260804-095507/probe-results/`. There IS a precedent, missed on the
  first pass: `tasks/20260716-123551/perf-results/` holds the v0.7.0 baseline
  as committed per-scene JSON plus `frametime.csv`, organized by render path
  (`sw/`, `xgpu/`, `combat/`, `web/`). Same shape, same reason - a generated
  artifact is only evidence if it outlives `target/`.
- RESOLVED - what "against the previous baseline" can mean, per series:
  - `asteroid_field`: COMPARABLE. `scene_baseline` still loads it, and
    `tasks/20260716-123551/perf-results/{sw,xgpu}/asteroid_field-{high,low}.json`
    exist. This is the one real release-over-release number.
  - `broadside-*`, `shakedown_run-*`: GONE. `broadside` is retired by
    `20260804-093910`; its perf series ends with it. Not a regression, but say
    so explicitly or the missing rows read as one.
  - `many_bodies`, `many_sections`, `many_projectiles`: NO PRIOR by
    construction. This run IS their baseline. Record them as such.
  The honest report is "one comparison, one retirement, three new baselines",
  not "the comparison is incomplete".
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
