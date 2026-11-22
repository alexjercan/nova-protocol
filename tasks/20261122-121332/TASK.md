# Role goals: hand in each step from anywhere for a reward

- STATUS: OPEN
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

## Done when
- Each role goal step waits for a Hand in from anywhere, pays its reward
  with the payout moment, the pane shows the chain, and the owner has
  played it.
