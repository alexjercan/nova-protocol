# Greeble design spike: a shared vocabulary across styles, more models

- STATUS: CLOSED
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

## Closure

Delivered 2026-08-16: GREEBLES.md (audit, class x style matrix, per-style art
direction, primitives-first sourcing plan, faction brainstorm, specs for the
catalog example and the bench blocks roster, ordered follow-up breakdown) plus
four bench renders, one per style.

Headline findings:
- The brief's hypothesis was wrong usefully: no model sharing - all 27
  fixtures own a generated mesh (scripts/gen-greebles.py recipes). The real
  gaps are CLASS COVERAGE and RULE STARVATION.
- Three signature pieces have zero reach on every hand-built bench subject:
  civilian_windows, armoured_sensor (industrial_radiator nearly so). The most
  characterful piece of each clean style never lands on what a player builds.
- On a plain thick run all four styles collapse to one edge-line rule each -
  the mechanical form of "not much variety".
- Design keys: armoured clean = SUPPRESSION vs civilian clean = FINISH;
  markings (ammo stripes) as the universal thin-shape carrier; a shared
  5-class core all styles implement; kit-cap ordering 8<9<10<11 as the
  tone-split test.
- Licence: Kenney Space Kit and Quaternius Ultimate Space Kit verified CC0;
  Kay Lousberg pack UNVERIFIED - check before use.

Follow-ups are enumerated in GREEBLES.md section "task breakdown"; opening
them is the owner's call.

## Owner re-ranking after the batches (2026-08-17)

The verdict the whole arc aimed at, after the four vocabulary batches
landed: "the new greeble kit looks good... I still think salvage >
industrial, but now armoured and civilian are closer to them." The gap the
original ranking named (armoured/civilian "pretty low kind of equal") has
closed; the order itself was never the target. No tuning-pass task filed -
the recorded tuning inputs stay in the batch task closures if a later pass
wants them.
