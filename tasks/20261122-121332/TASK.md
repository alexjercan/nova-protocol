# Role goals: hand in each step from anywhere for a reward

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.19.0, progression, jobs

## User facts
- Owner 2026-11-22: "right now there is nothing happening after you
  'complete' one of the 'mainline' jobs ... the job just goes away and
  nothing is shown, not even letting you to 'hand it in' or anything ... at
  least let you 'hand in' the job (anywhere obviously it is not station
  dependent) but maybe also gives you some credits and something like a
  feeling of progress/achievement".
- Owner approved the design page recommendations: "for the hand-in
  recommended sounds good".

## Decisions (owner 2026-11-22)
See `DESIGN-role-goal-hand-in.html` in this folder.
- H1: each met step waits for a Hand in button on its Jobs pane goal row,
  usable from anywhere.
- C1: steps live in content: `RoleGoal { title, steps: [RoleGoalStep { kind,
  reward_cr }] }` with `RoleGoalType::StationsOwned`; every base role lists
  it as step 2. **(breaking)** for mod start roles; the example mod migrates.
- R1: 3,000 cr for step 1, 10,000 cr for step 2, every base role.
- The Jobs pane shows the whole chain (done dim with play time, current
  lit, later dim). Map OBJECTIVES shows `done` in Accent while a step waits.
- Notices `GOAL-n done: hand in` and `GOAL-n PAID x CR`, payout sound and
  count-up, as a job hand-in. No music sting, no lasting HUD chip, no rank.
- E1: finite chain; the last paid step stamps the goal met.
- Full mod surface: `OnGoalStepDone`, `OnGoalStepHandedIn`, credit cause
  `GoalReward`.

## Agent findings
- Reproduction and the full map are on the design page (code at
  aae81b80a); evidence logs in `evidence/`.

## Delivery
Build the approved design per the page's End state, What dies and What
must fail loudly sections, with the playable example `role_goal_sandbox`.

## Verification
- The page's proof plan: unit, ECS, the reworked `system_role_goals`
  range, the playable example walk, frames on both themes.

## Build (2026-11-22)
- Shape: `RoleGoal { title, steps: [RoleGoalStep { kind, reward_cr }] }`;
  `StationsOwned` is a kind. `GoalRecord.paid_at` stamps each hand-in;
  the goal is met at the last. Lint and load refuse no step, a zero count
  and a zero reward; a Load refuses a record with more stamps than steps
  or stamps out of order or past the clock.
- Hand-in: `GoalHandInCommand` -> `plan_goal_hand_in` ->
  `apply_goal_hand_ins` (nova_interface jobs.rs). Pays from anywhere with
  cause `GoalReward`, the Paid cue and the balance count-up, raises
  `GoalStepHandedIn`; the scenario bridge fires `OnGoalStepHandedIn`.
  `GoalStepDone` fires when the current step turns met, never on a Load.
- Notice order (the page's "Not verified" item): goal lines wait. A goal
  line posted while a plain line shows queues behind it and shows when it
  ends; a later goal line replaces the queued one. So the bounty that
  meets step 1 shows `JOB-3 PAID`, then `GOAL-1 done: hand in`; a hand-in
  in the same pane visit shows `JOB-3 PAID`, then `GOAL-1 PAID`. Proof:
  `a_goal_line_waits_behind_the_job_paid_line_and_shows_after_it`.
- Found: the `GoalReward` `OnCreditsChanged` fires on the next fixed tick,
  so after `OnGoalStepHandedIn` and only once the pane shuts (the pane
  holds the clock). Both walks wait for it after the pane shuts. The
  `OnJobHandedIn` doc says its credit events come before it; same drain,
  not checked.
- Census (`CENSUS-BEFORE.md`, `CENSUS-AFTER.md`, step 1 paid at once in
  the greedy model): median of role medians, gain at 15 min, 4,113 cr ->
  4,474 cr; role band x0.80-1.10 -> x0.85-1.08. Step 1 is paid by 15 min
  on 0.4% (pirate) to 47.5% (transport) of seeds.
- Walk: `role_goal_sandbox` under `NOVA_AUTOPILOT=1`, 9 PASS lines and 9
  timeline markers; frames in `~/Pictures/nova-role-goals/`.

## Done when
- Each role goal step waits for a Hand in from anywhere, pays its reward
  with the payout moment, the pane shows the chain, and the owner has
  played it.

## Owner check (2026-11-22)
- Played role_goal_sandbox and approved: "works".
