# Retro: RCS mouse control delta-driven instead of virtual-joystick accumulate

- TASK: 20260718-185826
- BRANCH: feat/rcs-delta-control
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Small, honest scope. The playtest complaint ("way too hard to control") had
  a precise mechanical cause (accumulate persists after the mouse stops), so
  the fix was two coupled changes (SET + gated decay) and nothing else. No
  scope creep into retuning the whole feel.
- The gate design reused an existing marker (`RcsActive`) to keep the player
  and autopilot intent sources independent, so the autopilot terminal-settle
  from 20260718-122932 needed zero changes and zero risk. The one new flight
  test pins exactly that split (player decays, autopilot-proxy does not).
- Ran the full `flight::` + `input::player::tests::rcs` suites (77 tests), not
  just the two new ones, because the decay lives in the autopilot's FixedUpdate
  chain - the `changed-shared-observer` lesson applied and paid off (green, no
  regression).
- Both new tests were checked for the fail-without-fix property during review,
  and the player.rs test deliberately runs in a harness with no decay so the
  SET-not-accumulate assertion is isolated from the decay.

## What went wrong

- Nothing structural. The only friction was mechanical: a fresh-worktree cold
  rebuild made the single test run take ~5 min of compile before 5.6 s of
  tests. Not a process error, just the cost of an isolated worktree's cold
  target dir.

## What to improve next time

- This is a cycle where a spike's on-paper primary (held-direction) lost to its
  runner-up (delta) once it was in the hand. That is not a spike failure - it
  is what playtests are for - but it is worth encoding: when a spike decision
  rests on "which feels better", treat the first shipped version as a
  hypothesis and expect a possible reversal, and keep the deciding parameter a
  single tunable constant so the reversal is a one-line change (it was:
  `RCS_PLAYER_INTENT_DECAY`, `RCS_AIM_SENSITIVITY`).

## Action items

- [x] Bumped `changed-shared-observer-run-the-module-suites` in LESSONS.md (now
  x3 - moves to Pending promotions for the user to fold into a skill/AGENTS.md).
- [x] Added `playtest-can-reverse-a-spike-feel-call` to LESSONS.md.
- No follow-up code task: the seeded family follow-ups (cap ring 20260718-144939,
  ORBIT error-relative RCS 20260718-151102) remain OPEN and unaffected.
