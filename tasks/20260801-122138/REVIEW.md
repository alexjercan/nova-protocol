# Review: Fix shakedown an_early_derelict_kill_skips_to_the_fight failing on master

- TASK: 20260801-122138
- BRANCH: master

## Round 1

- REVIEWER: out-of-context (Parfit)
- VERDICT: APPROVE

No findings.

Verified:

- `HEAD^..HEAD` diff: one test-helper fix plus task record.
- Task steps and close-out match code mechanism.
- `nix develop --command cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight`
  passed.
- `nix develop --command cargo test -p nova_assets --lib` passed, 96 tests.
- `tatr check 20260801-122138` passed.
- `git diff --check HEAD^ HEAD` passed.

Could not verify:

- No separate generated RON read was applicable; scenario script unchanged, no
  generated content changed.
