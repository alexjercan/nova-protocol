# Retro: Re-scope spaceship system set gating to scenario-liveness

- TASK: 20260711-212519
- BRANCH: feat/scenario-live-gating (landed 953939d)
- REVIEW ROUNDS: 1 (APPROVE; 5 MINOR + 1 NIT, all addressed pre-landing)

## What went well

- Spike-first paid off exactly as intended: the spike had already answered
  WHY the old gate existed (preview sections carry live input bindings) and
  named the replacement signal (CurrentScenario), so implementation was
  mechanical and review found no correctness issues in the gate itself.
- The out-of-context review agent earned its cost again (fifth catch for
  the pattern): it found the stale prose in crates NOT touched by the diff
  (nova_menu, nova_assets), the test doc overpromising, and the one real
  behavioral delta (the orbiter's PD attitude hold now running in MainMenu)
  - all invisible to in-session eyes that had just written the diff.
- Factoring the gate as configure_scenario_gating (pause-gating precedent)
  made the tests production-faithful with zero extra plumbing, and the
  real-observer test (LoadScenario/UnloadScenario driving the gate) worked
  headless on the first try.

## What went wrong

- Stale-comment sweep was scoped to files the diff touched. The task step
  said "update comments citing the old gate", and I grepped only where I
  was already editing; nova_menu's and nova_assets' comments citing the
  editor gate - now describing the opposite of reality - were left for
  review to catch (R1.1, R1.2). Root cause: treated the sweep as "fix what
  I see" instead of grepping the workspace for the invariant's fingerprint
  ("editor gates the").
- The behavioral-delta audit stopped at "does anything panic or misbehave"
  and did not produce a written list of what NEWLY runs where (R1.5). The
  PD attitude hold on the menu orbiter is benign, but I did not know that
  until the reviewer named it; enabling systems in a new context deserves
  an explicit enumeration, not a vibe check.
- tatr-same-second-collision hit AGAIN during the spike phase (three
  `tatr new` calls in one && chain -> one shared ID, two tasks silently
  lost until the directory listing exposed it). Third occurrence; the tatr
  skill gotcha exists and did not prevent it because the command was
  composed without consulting it.

## What to improve next time

- When a change retires or relocates an invariant, grep the WORKSPACE for
  prose citing it (comments and docs, not just code symbols) before calling
  the change done; stale comments describing the old world are consumers
  too. This is the sweep-then-delete lesson extended from symbols to prose.
- A gating change's definition of done includes a written what-newly-runs
  enumeration per newly-enabled context, checked one by one.
- Never chain multiple `tatr new` in one command; the IDs are
  second-granular. Sleep or separate invocations.

## Action items

- [x] Ledger: bump out-of-context-review-pass (x5), sweep-then-delete
      (prose variant, x3 -> pending promotion), tatr-same-second-collision
      (x3 -> pending promotion), audit-state-gates-on-new-entry-path
      (enumeration variant, x2).
- [ ] The remaining two spike tasks (20260711-212521, 20260711-212504) pick
      up the delta list this cycle recorded (PD hold in MainMenu) as part
      of their visual verification.
