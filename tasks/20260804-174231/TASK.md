# probe run menu_scenarios flakes process_exit on a clean run

- PRIORITY: 0
- TAGS: backlog,bug,examples,testing
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -

## Story

`probe run menu_scenarios` intermittently reports `process_exit FAIL` on an
otherwise clean run: the process exits non-zero AFTER
`probe: script complete, exiting` and
`harness completion: all collectors done, exiting`, with `run_completed`,
`reached_playing` and `log_clean` all PASS and no panic or ERROR line anywhere
in `run.log`. Because `ui/` is a probe category, this makes
`cargo run -p nova_probe -- run ui` flaky, so any task carrying that command as
a DoD proof cannot rely on a single green run.

Measured while reviewing `20260804-094021` (2026-08-04, Xvfb :99):

| Tree | Failures |
|-|-|
| `feature/ui-pointer-driven` | 2 of 6 |
| `master` (a283846b) | 1 of 6 |

It reproduces on master at a comparable rate, so it is NOT the pointer-input
rebuild - `20260804-094021` only surfaced it by running the category often
enough to see it. Recorded there as review finding R1.2.

SUPERSEDED IN PART, 2026-08-04: the branch column above turned out to be a real
branch panic, not this flake - see the lead in NOTES. Only the master column
still belongs to this task, and it needs re-measuring with stderr captured.

## Steps

- [ ] Reproduce with the run's exit status captured directly
      (`DISPLAY=:99 NOVA_AUTOPILOT=1 cargo run --example menu_scenarios
      --features debug; echo $status`) in a loop, to confirm the non-zero code
      comes from the example process and not from probe's own accounting.
- [ ] Identify the exit code and its source. Both halves are open: a shutdown
      path in the app (wgpu/window teardown, an `AppExit::Error`) and probe's
      `process_exit` check itself.
- [ ] Determine whether other harnessed examples share it, or whether
      `menu_scenarios` is unique - it is the one run that launches a scenario
      and then reports its collector done from inside the walk.
- [ ] Fix it at its source, or - if it is an upstream teardown race outside our
      control - make `process_exit` distinguish it from a real failure and say
      so in the check's own text.

## Definition of Done

- [ ] `cmd:` `for i in (seq 10); DISPLAY=:99 nix develop --command cargo run -p
      nova_probe -- run menu_scenarios; or exit 1; end` - ten consecutive green
      runs.
- [ ] The cause is recorded in the task, not just the symptom.

## Notes

- Do NOT weaken `process_exit` into a warning to make this green; a non-zero
  exit is exactly what that check exists to catch.

### The lead, found 2026-08-04 while addressing R1.2

`run.log` DOES NOT CAPTURE PANICS, so `log_clean PASS` proves nothing about
them. Chase that first - the "clean run that exits non-zero" may simply be a
panic nobody is being shown.

Established on `feature/ui-pointer-driven`: three direct runs
(`NOVA_AUTOPILOT=1 cargo run --example menu_scenarios --features debug`) all
exited 101 with `thread 'main' panicked at examples/ui/menu_scenarios.rs:292`
in the captured stderr, while `grep -c panicked` over the matching
`probe-runs/*/menu_scenarios/run.log` returned 0 and probe reported exactly the
symptom in this task's Story: `process_exit FAIL`, every other check PASS.

101 is the standard Rust panic exit code. So:

- The measured rates above are not one phenomenon. The 2-of-6 on the branch was
  a real branch panic (a scenario row past the list's fold, fixed in
  `20260804-094021`); whether the 1-of-6 on master is a third-party teardown
  race or another swallowed panic is now the open question.
- Re-measure master with stderr captured directly rather than through probe,
  since probe's own report cannot currently tell the two apart.
- There is a second, arguably larger bug here: a harnessed run can panic and
  still report `log_clean PASS`. Whatever the exit-code cause turns out to be,
  probe should surface a panic in its checks instead of leaving it to an exit
  code the reader has to decode.

### A second symptom, found 2026-08-04 in review round 2 of 20260804-094021

`menu_scenarios` also fails probe with a TORN `timeline.jsonl`, on a run whose
log is otherwise spotless. Measured once in 7 `probe run` invocations on
`feature/ui-pointer-driven` (`DISPLAY=:99`):

```text
probe: menu_scenarios: timeline.jsonl: malformed timeline line 143
  menu_scenarios           ERROR    measured    -  11s
```

The run itself was clean: `probe: script complete, exiting`, `autopilot: cycle
complete, no panic`, `harness completion: all collectors done, exiting`, and
zero `panicked` or `ERROR` lines in `run.log`. Only the ARTIFACT is corrupt, so
the report never gets written and the example reports ERROR rather than a check
failure.

Line 143 splices two records that cannot come from one writer:

```text
{"data":null,"frame":168,"kind":"scen{"data":{...},"frame":168,"kind":"variable","name":"beat","scenario_elapsed":0.023779348,"t_real":4.790030831}
```

Same `frame`, but `t_real` 5.08 in the head and 4.79 in the tail - two
`ProbeTimeline` instances writing one path. `ProbeTimeline::create`
(`crates/nova_probe/src/recorder.rs:200`) uses `File::create`, which truncates
to offset 0, while the earlier instance's `BufWriter` keeps writing at its own
offset - so a second recorder built later in the same process overwrites the
head of the first's stream and leaves its tail. `menu_scenarios` launches a
scenario at the end of its walk, which is the obvious place a second recorder
would be constructed.

Suspected fix: make the recorder a single resource for the process lifetime, or
have `create` refuse to reopen a path an existing `ProbeTimeline` already holds.

Not attributable to `20260804-094021`: that diff touches no `nova_probe`
source, and the branch reproduced clean 3 of 3 afterwards against master clean
3 of 3.
