# Retro: Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

- TASK: 20260804-095507
- BRANCH: master (no feature branch - the deliverable is a run and its record)
- REVIEW ROUNDS: 1

## What went well

- **The full-fleet run earned its place.** Six preceding tasks each proved
  their own category with `probe run <category>` and all passed. Running them
  together surfaced two things none of them could: the `many_projectiles`
  spike profile and a shared-mechanism `click_named` flake that only shows up
  when enough driven UI examples run in one pass. The "run it all as one
  invocation" step was not ceremony.
- **Refusing to close on a red suite.** The task sat BLOCKED for a day on its
  own last step rather than record a green fleet beside a suite known to fail
  1 in 3. The block was written into the step with the failure rate and the
  blocking task id, so picking it back up cost nothing.
- **Filing instead of fixing.** Two findings, two tasks, neither absorbed into
  this one. The spike went to backlog; the flake went into the sprint at p84
  because it gated CI. Scope held.
- **Reading `checks.json` with `measured`, never alone.** This is what made
  the SKIPPED frame-time rows legible as policy rather than as gaps - 5/6 for
  correctness-only categories, 6/6 for `stress/`, and `screenshots/` not
  probed at all.

## What went wrong

- **"as CI will" was imprecise in the title and stayed there.** CI's fleet
  gate is `cargo test --test examples_smoke`; probe is in no workflow. The two
  runs prove different things and neither substitutes for the other. Caught
  during the run and tabulated in `NOTES.md`, but the title still says
  something the task does not do.
- **The closing evidence was gathered in the shared checkout.** Another
  session committed to `master` four times and ran Bevy examples on the same
  host while the fleet and the suite were running. The probe run is stamped
  `cafae048`, which is not the commit the run started from. The correctness
  verdicts survive this; the frame-time numbers do not really, and they were
  the one thing this task existed to record.
- **Probe's auto-baseline produced a comparison that looks like a finding.**
  With no `--baseline` given, probe picked the newest prior run - which was
  this task's own pre-fix capture from the same morning. Two categories came
  back WARN against it. Against NEW examples there is no meaningful prior, so
  the deltas mean nothing, but the aggregate verdict reads WARN and needed a
  paragraph of prose to defuse.

## What to improve next time

- **Pin the commit for any run whose output is evidence.** A sprout worktree
  at a fixed sha costs one command and makes the artifact attributable. In the
  shared checkout, capture `git rev-parse HEAD` before AND after a long run and
  compare - a silent mismatch is what happened here.
- **Pass `--baseline` explicitly, or expect the auto-pick.** For a first
  capture of new examples there is no honest baseline; the run should say so
  rather than let probe find one.
- **Frame-time capture wants a quiet host.** Nothing enforces this and nothing
  detects it. If the numbers matter, check for other running work first.

## Action items

- `20260805-091146` filed and left in backlog: `many_projectiles` p99 224ms
  against a 23ms median. Nothing gates on frame time - the only check is a
  baseline comparison - so it passed silently and needs a task to be seen.
- `20260805-091151` filed, sprinted at p84 and now CLOSED DONE (`87bcb956`).
  It is what unblocked this task's last step.
- No action taken on probe's auto-baseline behaviour. Picking the newest prior
  run is reasonable in general and wrong for a first capture of new examples;
  whether it should require an explicit `--baseline` is unclaimed and belongs
  to `nova_probe`, not here.
- Frame-time evidence gathered on a contended shared checkout is recorded as a
  caveat, not re-run. Owner call, made with the numbers in hand.
