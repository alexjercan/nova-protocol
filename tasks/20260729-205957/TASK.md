# End-of-sprint probe perf sweep (standing sprint-close check)

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, probe, perf, process

## Story

As a maintainer closing out a sprint, I want a standing end-of-sprint probe
sweep, so a perf regression introduced anywhere in the sprint is caught by the
sprint that caused it instead of by an ad-hoc comparison weeks later.

Task 20260729-002952 is the motivating case: a ~35% `scenario` frame-time
regression from the lazy `.ttc` font path went unnoticed until someone happened
to diff two probe reports. It was already fixed by then (`c3ee1988`), so the
investigation was archaeology. A routine sweep would have flagged it in the
sprint that shipped it, while the offending commit was still one of a handful.

This is a process task, not a code task: run the sweep, read the numbers, and
either sign the sprint off clean or open a regression task with the evidence.
Add one of these per sprint, tagged with that sprint's version.

## Steps

- [ ] From the sprint's final commit on the default branch, run the probe sweep
      for every scored example: `scenario`, `perf_baseline`, `playable`,
      `lifeline`. Artifacts land under `probe-runs/<short-commit>/<example>/`.
- [ ] Compare against the previous sprint's release-commit run set. Read
      `frametime.csv` (`mean_ms`, `p95_ms`) and the `fps_within_baseline` entry
      in `checks.json` - not the HTML reports.
- [ ] Sanity-check the host before believing any delta: a `p95_ms` far above
      the surrounding runs means a busy machine, not a regression. Re-run any
      example whose p95 is out of family. (See 20260729-002952 NOTES.md, where a
      whole run session read +5% purely from host noise.)
- [ ] Verify the run environment is comparable: `quality`, `resolution` and
      `backend` must match across the compared CSVs, and
      `grep -ricE '<enabled mod ids>' probe-runs/*/*/run.log` must be 0 unless
      mods are deliberately in scope.
- [ ] Record the sweep result in this task's NOTES.md: the two commits compared,
      the per-example table, and the verdict.
- [ ] If any example regressed beyond the 10% gate and the host was quiet, open
      a regression task tagged to the CURRENT sprint with the evidence, and
      attribute it by bisecting across the sprint's commits with commit-keyed
      run dirs.

## Definition of Done

1. notes: `tasks/<id>/NOTES.md` holds the compared commits, the per-example
   `mean_ms`/`p95_ms` table, and an explicit clean-or-regressed verdict.
2. cmd: the sweep commands are recorded and their `probe-runs/<commit>/` folders
   are committed.
3. manual: either the sprint is signed off perf-clean, or a regression task
   exists with the offending commit range named.

## Notes

- Motivating investigation: `tasks/20260729-002952/NOTES.md`.
- Commit-keyed `probe-runs/<short-commit>/<example>/` layout and automatic
  baseline discovery come from task 20260729-003352 (`00211ac5`).
- Native runs are sandboxed from the operator's profile since `a6d06220`, so the
  local `settings.ron` / installed mods no longer leak into a sweep.
- Use the `/probe` skill; it wraps the run and produces the reviewable report.


## Dropped

- REASON: old
