# Epic: civilization stance graph

- STATUS: OPEN
- PRIORITY: 49
- TAGS: v0.16.0,epic,stance

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
