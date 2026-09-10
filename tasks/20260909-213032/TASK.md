# Chapter one of season one: the Gantry rescue as a playable scenario

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.14.0, content, scenario, story

## Goal

The first chapter of the season-one story as a playable scenario in the base
game, under Scenarios as chapter one of the mainline campaign. Owner
(2026-09-09): "for this release I want to have at least the first scenario or
so of the story."

The story is the shared season-one outline
(`web/src/lore/seasons/season-1.md`) and the eighteen-page opening episode
(`web/src/comics/season-1/episode-1/`). The campaign brief that maps those
events to whole-ship objectives is step 4 of the story task
(`20260908-161328`); this task implements that brief and does not write story
of its own.

## The chapter

The opening, 2078, Saturn: Kaveri's crew at routine work out of Baikal, the
Gantry distress, the noncombat rescue, the funding refusal heard over comms,
the docked evacuation, and the homecoming. No combat, no pirates on screen.
The player operates the whole ship; interiors stay in the comic.

Content this needs, all in the block-ship language the base fleet uses:

- Kaveri, an unarmed civilian workship, as the player hull.
- Gantry, stranded and intact, with the surviving compartment readable from
  outside.
- Baikal as a static multi-cell structure the player departs from and
  returns to, and Ebro at its berth.
- A comms cast: Elena Ward at Baikal, Nadia Sen on Gantry, Daniel Reed's
  refusal from Keystone (voice only). Restrained lines, from the approved
  script, never copied from scenario prose the story rejected.
- Objectives built from the v0.12.0 primitives (`Sequence`, `once`, the
  pacing gaps in `nova_authoring::base_content::scenarios::pacing`): a work
  leg, the distress call, the transit, holding station beside Gantry for the
  evacuation, the return.

## Interaction needs

The brief lists what the rescue needs from the engine. Expected: hold
station at a mark for a timed beat, a cinematic on arrival, an objective that
waits on comms. If the brief needs a verb the engine lacks (a tow, a
transfer animation), stage it with what exists and record the gap on this
task; do not grow the engine in the release cycle.

## Proof

- The chapter authored in Rust under `nova_authoring` and generated with
  `content -- gen`; `content lint` green.
- A `system_` range in `examples/systems/` walks the chapter end to end on
  the autopilot and asserts each objective posts and completes in order, the
  victory outcome fires, and no error line is logged.
- A `screenshots/` loop of the arrival at Gantry for the news post and the
  store stills.
- The wiki and the news post describe the chapter with spoiler scope per the
  lore publication rules.

## Not in scope

Later chapters, weapons, the comic pages themselves, any story decision the
outline leaves open.
