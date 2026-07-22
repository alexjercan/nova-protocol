# Retro: broadside terminal act on player death

- Landed: e8940417 (squash), 1 review round, out-of-context APPROVE.

## What changed and why

Broadside's two player-death Defeat handlers (part one, gunship part two) fired
`Outcome(Defeat)` + `NextScenario` but never advanced `VAR_ACT`, while the
Victory handlers set `act = 2`. `CurrentOutcome` is last-write-wins, so a
mutual-destruction trade (the player's blast breaking the last corvette / the
gunship on the same beat the player dies) left the `eq_num(act, 1)` win gate
live, and the Victory overwrote the Defeat over the queued retry - a win from a
fight you died in. Fixed by `set(VAR_ACT, num(3.0))` first in both handlers,
which closes every win gate, mirroring lifeline's review R1.1 fix (act 3,
distinct from the win's act 2).

## How it went

Textbook bug cycle: reproduce-first paid off. Wrote the A/B trade pin
(`a_trade_after_the_players_death_cannot_overwrite_the_defeat`, both parts)
BEFORE the fix; it failed on the `act == 3` assertion against the unfixed tree
(act stayed 1, the trade flipped Defeat -> Victory), then went green after. The
reviewer independently sabotage-proved it (removed the terminal sets -> FAIL).
No findings; the fix mirrors an already-reviewed pattern exactly, so the diff
was small and the risk low.

## Self-reflection

- The `outcome-is-last-write-wins-close-the-act` lesson (already in the ledger)
  did its job as a class: lifeline caught it first (R1.1), the class was FILED
  against broadside as its own task, and this task closed the sibling. Filing
  the class rather than only fixing the motivating case is what made this a
  10-minute mirror instead of a rediscovery.
- I applied the fmt-before-commit reflex (the lesson from the prior flow) this
  time - ran `cargo fmt` before committing, no CI-bounce.
- Considered the review skill's trivial-diff carve-out (mirror of an approved
  pattern + fail-first pin) but ran the out-of-context review anyway because it
  is outcome-correctness logic and the user asked for the full cycle - cheap
  insurance, and the reviewer's sabotage-proof added real confidence.

## Follow-ups

- None. The other outcome handlers (shakedown, final_tally) were already
  terminal-act-clean (checked during the pacing/gravity work); this was the last
  known sibling of the R1.1 class in the mainline.
