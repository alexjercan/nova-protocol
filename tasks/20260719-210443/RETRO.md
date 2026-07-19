# Retro: fleet wiring (probe-all T2)

- TASK: 20260719-210443
- BRANCH: feature/probe-fleet-wiring (squash-landed as bbe3e9df)
- REVIEW ROUNDS: 1 (APPROVE; 1 gate finding fixed in-round, 2 NITs)

## What went well

- The exit gate earned its name: "run --all and READ every report" found
  the one thing every earlier layer had missed - perf_baseline had no
  exit path without the capture armed. Unit tests could not see it (it
  is a composition property of example + env), the smoke suite could not
  see it (perf_baseline is not smoked), and the spike ASSUMED it away.
  One real fleet run falsified the assumption in 184 seconds.
- Zero invariant false positives across 20 examples validated a design
  bet made back in the invariants task: bounds pinned to what the ENGINE
  guarantees (not scenario-tuned thresholds) generalize without retuning.
  The task explicitly predicted false positives at fleet scale; reality
  was kinder because the bounds were engine-honest from the start.
- The exit-ownership fix is a pattern worth remembering: two harnesses
  that can both end an app must be armed EXCLUSIVELY (`!perf_armed()`
  gates the autopilot) - ownership by construction beats ordering
  by luck.
- The anchor-scripted wiring (assert-counted anchors, per-shape lists)
  placed 55 plugin lines across 19 files with two correctable surprises
  (a comment-based false anchor; four comment-splitting inserts), both
  caught by reading the produced text before committing - the
  render-output-eyeball habit at file scale.
- The stacked-T3 flow (user-directed) cost nothing: T2's gate ran in its
  own worktree while T3's edits landed in another; the one cross-branch
  fix (perf_baseline) merges forward cleanly because T3 never touched
  that file.

## What went wrong

- The task body said "the 16 unwired examples" but enumerated 17 - a
  count/enumeration mismatch nobody caught at filing or planning. The
  enumeration governed (correctly), and the close-out records the
  discrepancy; counting things twice in a task body invites exactly this.
- The first wiring script treated a COMMENT mentioning nova_autopilot as
  the anchor (com_range) - the assert caught it mid-run, leaving six
  files edited and eleven not, which required a resume script with
  corrected lists rather than a clean rerun.

## What to improve next time

- Fleet-scale mechanical edits: dry-run the anchor scan FIRST (print
  matches per file, apply nothing), then apply - the probe-pass habit
  from doc sweeps applies to code sweeps unchanged.

## Action items

- [x] T3 (20260719-210450) resumes on the synced stack.
- [ ] R1.2 carried: perf/ plain-run rows measure 5/6 via the autopilot
      path; the fps column's skip keeps the surface honest - revisit
      only if it misleads in practice.
