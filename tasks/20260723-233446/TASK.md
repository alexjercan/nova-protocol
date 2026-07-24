# HUD: friendly/enemy allegiance marker over ships (small triangle/chevron above each entity)

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.9.0,hud,gameplay

## Story

Playtest of the ch5 raid (task 20260723-182855) surfaced this: with two AI
wingmen on the player's side and four enemy fighters in the same brawl, it is
hard to tell friend from foe at a glance. Add a small world-anchored marker
(e.g. a triangle/chevron above each ship) coloured by allegiance so the player
can read the fight instantly.

## Notes / pointers

- Allegiance is the `Allegiance` enum (`crates/nova_gameplay/src/relations.rs`):
  Player / Enemy / Neutral. The marker colour maps off that (and "no allegiance"
  = bystander/neutral).
- This is a HUD/overlay feature - look at the existing HUD instruments under
  `crates/nova_gameplay/src/hud/` (e.g. the targeting radar, target inset,
  maneuver instruments) for the world-to-screen anchoring pattern already in use.
- Scope questions for whoever picks it up: mark ALL ships or only within radar
  range? Only combatants (skip Neutral)? Billboard above the hull vs a radar-only
  chip? Keep it cheap - it draws every ship every frame (see the ch5 perf task
  20260723-233453; do not make the fps worse).
- A small runnable example / a scenario with mixed allegiances (ch5 itself) is a
  good visual test bed.

## Definition of Done (sketch - refine when picked up)

- Each ship shows an allegiance-coloured marker; friendly vs enemy is readable
  at a glance in the ch5 raid; the marker follows the ship and hides on death.
- test/example: a visual example or HUD test that exercises the mixed-allegiance
  case; no measurable fps regression.
