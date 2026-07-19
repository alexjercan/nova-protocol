# Retro: unified run report

- TASK: 20260719-112304
- BRANCH: feature/probe-run-report (squash-landed as 605335db + fixture fix dac8bb4a)
- REVIEW ROUNDS: 1 (APPROVE; R1.1 ANSI-strip fixed in-round)

## What went well

- The assembly task assembled: T2-T4's outputs plugged into the report with
  zero rework of the producing layers - the family's contract-first design
  (every artifact independently parseable, every inherited review note
  written down with an owner) paid out exactly here.
- The five inherited review notes were treated as REQUIREMENTS, each traced
  to code + a test/artifact at review time - none were lost across the four
  intervening cycles, because they lived in REVIEW/RETRO files, not memory.
- Two honest re-cuts instead of quiet compliance: the "plant foreign
  frametime/trace into the e2e run dir" clause was dropped WITH recorded
  reasoning (a fabricated run dir is a fabricated run), and the wasm break
  was caught by reading each background step's exit code individually
  rather than trusting the job's overall success.
- The one mid-cycle test failure was a rig bug (non-unique replace anchor
  doubling planted violations) caught by its own LITERAL expectation -
  x4 != x2 - vindicating literal-value pins over shape assertions.

## What went wrong

- The global `*.log` gitignore silently swallowed the run.log FIXTURE
  during the worktree's `git add -A`; the squash landed without it and the
  worktree (with the only copy) was already deleted. A fresh clone's CI
  would have failed the healthy-fixture test. Caught only because the
  squash diff was READ file by file at land time; fixed on master with the
  file + an ignore exception for crate test fixtures.
- The wasm gap (new native-only bin, no stub main) repeated a shape T2
  already solved for the recorder module - the bin dimension of the same
  rule was not carried over.

## What to improve next time

- After staging fixtures, verify each file is TRACKED (`git ls-files` the
  fixture dir, count files) - `git add -A` reports nothing about what the
  ignore rules dropped.
- New native-only bin => stub wasm main in the same edit as the bin
  registration (the module cfg is not enough; bins compile independently).

## Action items

- [x] Lesson added: fixture-adds-verify-tracked; .gitignore exception carved
      so the class dies at the tool level.
- [ ] T6 (the runner CLI) now has its full contract: produce the run dir
      (timeline + invariants + log always; frametime/trace per pass) and
      call run_report at the end.
