# Whole-ship prototypes: assets/ships/*.ron placed by id with ship-level modifications

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.6.0,modding,scenario,folded

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 3, phase 2): the Wesnoth `unit`-level analogy - whole ships as
`assets/ships/*.ron` templates (a `SpaceshipConfig` built from prototype-referencing
sections), which a scenario places by id with ship-level modifications. Same
resolve mechanism as step 2, one level up.

## CLOSED (folded into 20260714-113418, 20260714)

Decision (user, during /flow 113414): FOLD this into the typed-multi-file-bundle
work (113418) rather than build a standalone ship catalog now. Two reasons surfaced
while planning:

1. NO built-in ship is reused (each scenario places at most 2 ships, each once), so a
   ship catalog RELOCATES each ship's config into a `ships/*.ron` file rather than
   deduplicating it - low immediate payoff, unlike the section catalog (sections ARE
   reused across ships).
2. It overlaps the bundle model: in 113418 a bundle is a pile of TYPED content files
   merged by kind (ship / section / scenario / ...), so "ships as data" is naturally a
   content kind there. Building a standalone ship catalog + loader now would just be
   reshaped by the bundle loader.

The ship-prototype MECHANISM (a `ShipSource` = `Inline(SpaceshipConfig) |
Prototype(ShipId)` resolved at spawn against a `GameShips` catalog, plus ship-level
component modifications mirroring 113411's `SectionModification` model) is still
wanted - it is now designed and delivered as part of 113418, where "ship" is one of
the bundle's content kinds. No standalone assets/ships catalog; no built-in re-port
(the ships stay inline until a fleet scenario reuses one or the bundle port relocates
them). Nothing shipped for this task; the work moved, it was not dropped.
