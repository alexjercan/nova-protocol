# DECISION - the objective reveal card is deleted, not dormant

- STATUS: ACCEPTED

## Context

Owner playtest (2026-07-29): "the objective still appears first as a big thing
on the spaceship HUD, lets leave it only in the top middle as a 'chat message'
like popup". The big thing is `hud/objective_reveal.rs` (task 20260721-211520):
a rotated 1.35x cockpit card that appears (0.35s), holds (2.3s) and tucks
(0.55s) into the objective stack, after which - and ONLY after which - the chip
appears and pops (`pop_chip_on_reveal_tuck`, task 20260729-163816).

The card is not just a visual: the stack's chip lifecycle is GATED on its tuck
(`hand_over` / `handed_over`, with a `REVEAL_TOTAL_SECS` fallback for postings
that get no card). So "stop showing the card" is not a one-line change, and the
two candidate artifacts are mutually exclusive in what they leave behind.

Two routes:

1. **Delete the module.** Remove `objective_reveal.rs`, its plugin, its spawn
   call, `ObjectiveRevealTucked`, the stack's handover machinery, and
   `NovaOsTabAnchor` (the card is its only consumer).
2. **Keep it dormant.** Stop calling `spawn_objective_reveal` but leave the
   module, the message, the anchor and the handover gate in the tree.

## Decision: delete the module (route 1)

Confirmed by the owner at the 2026-07-29 plan gate.

- Route 2 cannot stand on its own: with no card ever spawned, no tuck ever
  arrives, so every posting would wait out the `REVEAL_TOTAL_SECS` fallback
  before its chip appeared - the opposite of the "chat message" immediacy the
  owner asked for. The handover gate has to go either way, and once it does the
  module has no consumer.
- It leaves no dead code, no unconsumed `NovaOsTabAnchor` resource and no
  dead-code warnings (`warnings-clean-before-land`).
- The card is recoverable from git history if it is ever wanted back; a dormant
  module is not free - it keeps being read, swept and maintained.

## Consequences

- Supersedes the presentation half of task 20260721-211520 (diegetic objective
  reveal). That task stays CLOSED as history; its record is not rewritten.
- The chip stack (20260729-163816) becomes the sole presentation of a posting:
  the chip spawns and pops on the posting frame, and its read dwell runs from
  the posting rather than from the handover.
- CHANGELOG [Unreleased] lines that describe the card and "pops as the card
  tucks in" are rewritten from the final diff.
