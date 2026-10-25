# Epic: civilization stance graph

- STATUS: CLOSED
- PRIORITY: 49
- TAGS: v0.16.0, epic, stance

## User facts
- Stage 7 of the game expansion. Decision 12 (2026-10-12): a per-civilization stance graph (Ally, Neutral, Enemy; never a float reputation), by proximity: edges only between neighbouring civilizations, one-hop propagation, a way back; explored in a research note before any code, built only after the note is accepted and after Stage 2.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Stance:` title prefix and the `stance` tag.
1. Research note.
2. Implementation after the note is accepted.

## Done when
- Looting three ships of one civilization makes its station refuse the player and its rival's station sell an escort; a flip never reaches a civilization beyond the cluster radius.

## Children
- `20261012-100035` Stance: research note
- `20261012-100036` Stance: implement the graph after the note is accepted

## Owner feedback 2026-10-16
- Epic direction approved. Observed: some neutral ships do not retaliate (the escorts for sale). Part of this epic: attacking a neutral ship turns its whole group Enemy until the stance clears after some time or by some other rule.

## Closure check 2026-10-25
- Claim: the three-step offence, civilization-wide provocation, one-hop neighbour propagation, refusal/redemption and Ally escort eligibility meet the owner direction. `crates/nova_world_base/src/stance.rs` records `ShipProvoked` against the civilization for 900 s, then syncs its effective side to its generated hulls; `civilizations.rs` limits graph edges to neighbours. The closed implementation child holds the original player-flow evidence.
- Verification on the current shared tree: `cargo test -p nova_world_base --lib stance` (6/6), `--lib civilizations` (11/11), and `cargo test -p nova_ship --lib threat` (27/27), including the named player-hit provocation assertions. No live looting/escort purchase or fresh capture was run during this closure pass; those assertions are not claimed as new live proof.
- The later chart ADV chip is separate UI work and does not change this epic's graph acceptance.
