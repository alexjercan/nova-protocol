# Greebles: research a clearer placement language than region, density, orientation

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.16.0,skin,research

## User facts
- Owner feedback 2026-10-13 on the style language (`FixturePlacement { region, density, orientation }`):
  - "still not really happy with the language for styles"; "not entirely happy with `FixturePlacement` I think we might be able to do a better job at it".
  - `region`: "feels pretty difficult to understand what it means ... it's weird to take a region and actually understand where it will be placed, it's pretty random".
  - `density`: "interesting and makes sense, it adds like how rare this greeble is, but maybe with a better data structure/algorithm ... it won't be needed".
  - `orientation`: likely goes away if "plating accommodates the greeble" lands; the greeble should follow the plate's own axis by default (as `armoured_ammo_stripes` with `orientation: Along` does) and turn with an angled plate.
  - "we might want to do some research and thinking on this topic before we make changes".

## Agent findings
- Research only. No code until the owner accepts a note.

## Delivery gate
- A research note in this task folder: how placement works today (region, density, orientation, with real style content), what modders find unclear, prior art (procedural greebling in other games/tools), and two or three candidate languages with real RON examples for the existing styles. Stop for the owner.

## Verification
- The owner picks a language; the pick is recorded here.

## Done when
- An accepted placement language that the plating-accommodates-greeble task can implement.
