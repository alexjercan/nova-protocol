# Greebles: research a clearer placement language than region, density, orientation

- STATUS: CLOSED
- PRIORITY: 95
- TAGS: v0.16.0, skin, research

## User facts
- Owner feedback 2026-10-13 on the style language (`FixturePlacement { region, density, orientation }`):
  - "still not really happy with the language for styles"; "not entirely happy with `FixturePlacement` I think we might be able to do a better job at it".
  - `region`: "feels pretty difficult to understand what it means ... it's weird to take a region and actually understand where it will be placed, it's pretty random".
  - `density`: "interesting and makes sense, it adds like how rare this greeble is, but maybe with a better data structure/algorithm ... it won't be needed".
  - `orientation`: likely goes away if "plating accommodates the greeble" lands; the greeble should follow the plate's own axis by default (as `armoured_ammo_stripes` with `orientation: Along` does) and turn with an angled plate.
  - "we might want to do some research and thinking on this topic before we make changes".

## Agent findings
- Research only. No code until the owner accepts a note.
- Note landed 2026-10-14 in 0a46d2f95 (`RESEARCH.md`, `RESEARCH.html`). Three candidates: A (`on` a plate kind plus `pattern: Line | Every(n) | Ends | Middle | Once`; no density or orientation), B (a weighted table per plate kind with a bare weight), C (panels first). The agent recommends A. Awaiting the owner's pick.
- Owner decision 2026-10-14: all three candidates are REJECTED, and so is the in-game framing of the language ("Kinds a reader can see"). New direction:
  - Greebles get link points, like sections do.
  - The ship's outer shell is a 2D surface. Each greeble, and each empty cell, has rules for how it links to its neighbours.
  - A deterministic, seeded WFC-style solver places them. Line-shaped greebles can form lines, and a greeble can require empty space around it.
  - Rules may look past direct neighbours: 2x2 or 3x3 windows, or what lies within a range.
  - The language is a technical solver configuration. It uses no game-fiction words; the code is the code.
  - The research note is redone on this basis.

- Second note landed 2026-10-14 in 7dae7ad3c (seeded solve on the shell surface; candidates A, B, C; agent recommended C).
- Owner decision 2026-10-14: candidate C's structure is accepted, but its plate filters and socket keywords are not. The accepted language is greebles with four link points naming style-declared link types (`connects`), empty as a greeble with `fixture: None`, optional `near` (`greebles`, `radius`, `factor`, where 0 is a hard ban), optional `max_count`, required `rotate` and `weight`, and no plate geometry filters. It is recorded at the top of `RESEARCH.md` under "Accepted language". No compatibility with today's look.
- Orchestrator close 2026-10-14: research accepted; the solver slice starts under 133643.

## Delivery gate
- A research note in this task folder: how placement works today (region, density, orientation, with real style content), what modders find unclear, prior art (procedural greebling in other games/tools), and two or three candidate languages with real RON examples for the existing styles. Stop for the owner.

## Verification
- The owner picks a language; the pick is recorded here.

## Done when
- An accepted placement language that the plating-accommodates-greeble task can implement.
