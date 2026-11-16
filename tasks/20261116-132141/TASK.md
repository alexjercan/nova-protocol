# Job rewards that scale with risk and distance, and a payout moment

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.19.0, jobs, economy

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- Agrees that jobs are boring because rewards are flat.
- Agrees that a payout needs a moment. Owner wants a clean, minimal
  presentation: no camera effects for this.

## Agent findings
- Reward formulas (`crates/nova_gameplay/src/jobs.rs:118-144,2005-2030,2234`):
  fee = 1-5 x 50 cr, doubled for Distress, Neutralize and Supply. Procure
  ask x1.5 plus fee; Recover ask x2; Escort half the cargo (300-2400) plus
  150 per raider plus fee; Distress 200 plus 2x fee; Supply 200 plus goods
  x1.5 plus 2x fee; Neutralize 400 plus 2x fee, for any hull size.
- No distance or advancement scaling. Only the escort raider count scales
  with advancement (`jobs.rs:145,1485-1509`).
- Hand-in writes one pane line and plays one click
  (`crates/nova_interface/src/jobs.rs:2116-2124`).
- Every role goal opens with "own one station"
  (`crates/nova_world_base/src/goal.rs:4-8`); price is 2x a 10k-40k worth
  (`station_layout.rs:136`, `station_buy.rs:62`).

## Delivery
1. Reward terms for distance (leg length), danger (target hull class, raider
   count) and advancement. Show the terms on the job card so the player sees
   why a job pays more.
2. Payout moment: a short HUD notice with the amount, a credit count-up in
   the pane, and a payout sound. No camera motion.
3. Retune and record a before/after pay table per job kind.

## Verification
- Unit tests for the reward formula terms.
- Pay table before and after, per job kind, at advancement 0 and 0.5.

## Done when
- Riskier and longer jobs pay visibly more, the card shows why, and a
  hand-in has a clear payout moment.
