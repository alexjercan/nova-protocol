# Retro: entity-filter id/other_id docs + scenario load/test rework

- TASK: 20260715-212600
- OUTCOME: shipped (landed dacf5d3a); review APPROVE, 1 NIT fixed.

## What went well

- The task shipped with a pre-grounded spec (the code sweep ran when the task was
  filed), so implementation was mostly writing - and re-verifying the two
  load-bearing claims at review (area.rs fires id=area/other=entrant; the spawn
  action takes `_info`, underscored=unused) took two `sed`s.
- The user's mid-task expansion ("section 7 is trash") turned out to be a real
  accuracy bug, not just polish: the old guide told authors to test with
  `08_scenario.rs`, which builds its config in Rust and cannot load a RON file.
  Verifying that (reading the example's `showcase()` fn) confirmed the cut.
- Chasing the "how do you actually test a scenario" question surfaced that there
  is no scenario picker yet (task 200828 still OPEN) - so the honest answer is a
  workaround (repoint NEW_GAME_SCENARIO_ID / chain via NextScenario), which the
  docs now state plainly instead of implying a clean path.

## What went wrong

- First `npm run ci` failed with `prettier: command not found` - a fresh sprout
  worktree has no `node_modules`; I had skipped `npm install` in it. Cheap fix,
  but a reminder that every new web worktree needs an install before ci.
- The `pkill -f "...8095"` to stop the dev server kept returning exit 144 and
  aborting the rest of the command line (it kills within its own process group).
  Left background dev-servers running instead of cleanly stopping them.

## Lessons

- `docs-claims-are-code-claims`: a how-to that names a test path or a tool is a
  factual claim about the code and must be verified like one - "test it with
  example X" was wrong because X builds its input in Rust, not the authored
  format. Read the tool before documenting it as the way to test.
- `npm-install-per-web-worktree` (x1): a fresh sprout worktree has no
  node_modules, so `npm run ci` dies on a missing binary until you `npm install`
  in that worktree's web/ first. (See also worktree-shares-main-target for the
  cargo analogue.) 20260715-212600.
- Stop a background webpack-dev-server with TaskStop (when launched via the tool)
  rather than `pkill -f` from a Bash call - the pkill takes out its own group and
  reports failure.
