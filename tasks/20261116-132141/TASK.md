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

## Decisions (owner said start 2026-11-18; agent calls, stated; map `MAP.md`)
- Model: a job's reward is a sum of named terms, kept on the job:
  `JobReward { base_cr, goods_cr, range_cr, threat_cr }`, and `Job` gains
  the posting facts `leg_m` and `threat`. `Job.reward_cr` dies. Save layout
  changes; no v0.18.0 save loads this release already.
- Advancement factor `A = 1 + 2 x advancement` of the poster scales the
  base and the range rate. The fee (50-250 cr) joins the base once; the
  doubled fee for Distress, Supply and Neutralize dies.
- Range: 30 cr x A per km of `leg_m`. Escort, Distress and Supply draw
  their leg per job, 6 to 16 km in 2 km steps (was a fixed 10 km).
  Recover and Neutralize read the metres from the poster to the body at
  posting. Procure has no leg.
- Threat: the weapons the job puts against the player, 300 cr each.
  Escort: raiders x turrets of the band. Neutralize: the live hull's
  turrets, torpedo bays and railguns. An unarmed target pays no threat.
- Base by kind (x A): Procure 0, Recover 100, Escort 200, Distress 200,
  Supply 200, Neutralize 300.
- Goods: Procure and Supply keep goods at ask x 3/2. Escort pays a quarter
  of the cargo (was half). Recover pays half the stack at ask, at most
  2,000 cr (was twice the stack, up to 139,730 cr at advancement 0.5); the
  player still keeps the stack.
- Authored rewards (visit profiles, scenario `PostJob`) stay one flat
  figure: base only. No content format change.
- Station funds: a thawed station below its start balance regains credits
  by the item restock rule (time frozen). Today nothing refills a station,
  so it pays 10 to 20 hand-ins and then refuses for ever (`MAP.md` G).
  `NO FUNDS` stays for a station drained in one visit.
- `RESCUE_FEE_CR` gets its own 200 cr constant; the SOS bill does not move.
- Card: the inspector lists each non-zero term under Reward as a number
  row (Base, Goods or Cargo, Range with km, Threat with the weapon count);
  a one-term reward shows no rows. The board list gains Range and Threat
  columns. No sentences (owner UI rule 2026-11-17).
- Payout moment: one new `Payout` sound at the hand-in (replaces the
  click); the pane head balance counts up on real time with a `+N` figure
  beside it for a short dwell; a HUD notice `<code> PAID <n> CR`, silent,
  seen when the HUD next shows. No camera motion.

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
