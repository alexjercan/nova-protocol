# Epic: regional economy, clock and restock

- STATUS: OPEN
- PRIORITY: 59
- TAGS: v0.16.0,epic,economy

## User facts
- Stage 5 of the game expansion. Decisions 2026-10-12: authored ask and bid are reference prices; actual price = reference x scarcity x demand from content rules (D8); a play-time clock joins the save when restock lands, with a catch-up rule per frozen body type (decision 8); a station of a robbed civilization refuses service for a play-time window (decision 10).
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Economy:` title prefix and the `economy` tag.
1. Computed prices at dock time.
2. Play-time clock in the save.
3. Restock and the robbery refusal window.

## Done when
- Prices differ between clusters for a reason the player can read, a station restocks after time away, and robbing a trader costs service at that civilization's stations for a while.

## Children
- `20261012-100027` Economy: computed prices from reference, scarcity and demand
- `20261012-100028` Economy: play-time clock in the world save
- `20261012-100029` Economy: restock and the robbery refusal window
