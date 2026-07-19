# Retro: depth markers (probe-all T3)

- TASK: 20260719-210450
- BRANCH: feature/probe-depth-markers (squash-landed as b4f7982e)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs)

## What went well

- The design-promised bar did real filtering work: every monotonic
  candidate was REJECTED on inspection (broadside's `act` resets on
  Retry - a fact one file-read established), and what remained (markers
  at existing assertion sites) is flake-proof by construction because
  nothing new is promised. The goldens lesson, applied preventively.
- The stacked flow (user-directed) delivered its promise: T3 was
  implemented and compiled while T2's 40-minute exit gate ran, T2's one
  fix merged forward without conflict, and validation ran against the
  post-merge state - zero wall-clock wasted, zero stack-induced rework.
- Validation doubled as feature exercise: `probe run sections,broadside`
  was the first real MIXED category+name spec, and the marker
  verification (grep each timeline for each name) proved emission
  once-ness, order (the torpedo chain), and completeness (11/11 stages)
  in one pass.
- The broadside buffer-and-flush shape (state out of the world in
  `advance`) solved the borrow problem without restructuring the script -
  a pattern worth remembering for world-removed resource closures.

## What went wrong

- Nothing failed. One friction: the per-file marker designs needed real
  reading of seven harness closures (no shortcut existed) - the "judged
  per example at implementation" scoping in the task was honest about
  exactly this cost.

## What to improve next time

- When stacking, note the stack shape in each task's worktree at branch
  time (this cycle did it in prose in the close-out; a one-line STACK:
  header field would make it greppable).

## Action items

- [x] Probe strand T-family complete: T1 aggregate, T2 fleet wiring,
      T3 depth. The close-out task (20260719-211500) is the last layer,
      already stacked and in flight.
- [ ] R1.2 carried: compare broadside timelines by `stage N` names, not
      the note text.
