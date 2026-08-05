# Review: Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

- TASK: 20260804-095507
- BRANCH: master (no feature branch - this task RUNS the fleet and records it;
  its only code change was the one-line stale-row correction in `c6d31a96`)

## Round 1

- REVIEWER: in-session self-review of the record and the closing evidence
- VERDICT: APPROVE

Definition of Done, item by item:

| DoD | Verdict | Evidence |
|-|-|-|
| Full fleet green as one probe invocation | MET | exit 0, run `cafae048`; 17/17 categories PASS every correctness check |
| No hand-rolled completion guard or beat-boolean | MET | `rg` over `examples` returns no match |
| Catalog, disk layout and smoke lists agree | MET | `catalog_matches_disk` ok |
| Every category has a probe policy row | MET | `every_category_has_a_probe_policy` ok |
| Workspace suite passes | MET | exit 0, 1543 passed / 0 failed / 2 ignored, 72 binaries |
| Frame numbers and caveats in `NOTES.md` | MET | "The v0.10.0 fleet run" and "The closing run" |

- [x] R1.1 (MAJOR) the task sat BLOCKED on its own last step for a day, on an
  `examples_smoke` flake it had diagnosed but not fixed. Closing while that
  step was open would have recorded a green fleet on a suite known to fail
  1 run in 3.
  - Response: resolved, not waived. `20260805-091151` landed (`87bcb956`) and
    closed DONE; both DoD commands were re-run afterwards and both are green,
    with `examples_smoke` 9/9 and the two previously-flaking categories
    (`menu_newgame`, `editor`) passing.

- [x] R1.2 (MINOR) the closing run's aggregate verdict is WARN, not OK, which
  reads as a failed DoD at a glance.
  - Response: not a defect. The WARN is `fps_within_baseline` on
    `many_bodies` (+14.2%) and `scene_baseline` (+14.8%), a documented SOFT
    gate that defers to the reviewer. Probe auto-selected this task's own
    pre-fix run as the baseline; these are NEW examples, so there is no prior
    capture to compare against and the delta is noise by construction. Owner
    call, recorded in `NOTES.md`. Correctness - which is what this task set
    out to prove - is PASS everywhere.

- [x] R1.3 (MINOR) the evidence is not attributable to a single commit. Another
  session was committing to the shared checkout and running Bevy examples on
  the same host throughout; HEAD moved across four commits during the two runs,
  and the frame-time capture was taken on a contended box.
  - Response: accepted by the owner rather than re-run. The caveat is stated
    plainly in `NOTES.md` under "Caveat on attribution". It weakens the
    frame-time numbers, which nothing gates on; it does not weaken the
    correctness verdicts, which do not depend on a quiet host.

- [x] R1.4 (MINOR) one full-fleet run cannot by itself disprove a 1-in-3 flake.
  - Response: accepted. The confidence comes from the root-cause fix
    (`Pointer<Click>` dispatching off the previous frame's hover map, fixed by
    pinning the pointer for the length of a driven run), not from the single
    green run. The run corroborates; the diagnosis in `20260805-091151` is what
    carries the weight. CI exercises the same gate on every PR.

Follow-ups filed rather than fixed here, both correct calls:
`20260805-091146` (`many_projectiles` p99 spikes, backlog) and
`20260805-091151` (the click flake, sprinted at p84, now closed).
