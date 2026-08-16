# Greeble design spike: a shared vocabulary across styles, more models

- STATUS: IN_PROGRESS
- PRIORITY: 62
- TAGS: v0.11.0,research,art,skin

## Goal

Design spike, owner-requested. No engine code changes; the deliverable is a
design document plus specs for follow-up tasks.

## The owner's brief

Ranking of the current looks: salvage (scrap) first, industrial a close
second, then armoured (combat) and civilian "pretty low kind of equal (they
have not that much variety)".

The ask, in the owner's words:

- Make the styles more "similar": "they should all have similar objects but
  looking different and having different 'functionalities'" - quotes because
  fixtures are visual only, they do nothing.
- Add object classes: "antenna, wires, cables, tubes, vent pipes, wheels and
  cogs", "ammo stripes", "even random things like batteries", "depending on
  each style/faction".
- Tone split: "combat and civilian have to look clean and official, but scrap
  can go ham and add all sorts of things" - which is why scrap wins today.
- Faction framing: "we can even think in terms of factions at some point if
  the mainline campaign grows", and the block ships could become "the flagship
  for ships instead of the kenney models - which I am starting to like more
  and more as an idea".
- Examples: "an example that presents the available greebles is a good idea",
  plus "expanding the bench one to have a lot more shapes and 'building
  blocks'" where a building block means "well this thing looks nice, it can
  be used for an actual ship".

## Deliverables

1. GREEBLES.md in this folder:
   - audit of what exists: every fixture per style (nova_authoring styles.rs)
     and every model on disk, with renders of each style on the bench
   - a cross-style vocabulary matrix: object classes x four styles, each
     class interpreted per style (same object, different look and pretend
     function), plus per-style exclusives
   - per-style art direction: what clean/official means for armoured and
     civilian, how far ham goes for salvage
   - a sourcing/production plan for new models: kitbash, packs, primitives;
     licence positions (link tasks/20260815-231945/PLATING-AND-GREEBLES.md,
     the single point for market research)
   - the faction angle: styles as factions, block ships as the mainline cast
     - brainstorm only, no commitment
2. Spec for a greeble catalog example (present every available greeble,
   parts-preview style with idle orbit).
3. Spec for the bench expansion: a building-blocks roster of larger shapes
   worth judging as real ships.
4. A recommended breakdown into follow-up tasks, effort-sized.
