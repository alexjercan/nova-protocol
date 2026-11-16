# SOS: a stranded ship can call for rescue and continue

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.19.0, survival, rescue

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- A crippled ship needs a way out: "calling for SOS to be saved or
  something", next to field repair.
- Owner rule on record: no player Neutralized ending; the only run end is
  `OnDestroyed` (`tasks/20261012-095526/DESIGN.html`). This task keeps that
  rule: SOS continues the run, it does not end it.

## Agent findings
- Neutralize: armed ship with no working weapon or no flight computer;
  unarmed ship with no thruster (`crates/nova_gameplay/src/integrity/neutralize.rs:157-223`).
  The marker is inserted once and never removed (`neutralize.rs:53-57`).
  Nothing brakes the player (`crates/nova_ship/src/flight/neutralized_brake.rs:11-43`).
  AI drops a neutralized target (`crates/nova_ship/src/input/ai/acquisition.rs:200-201`).
  Saves still write (`crates/nova_world_base/src/save/session.rs:439-452`),
  so a stranded state can be saved.
- A dry (no ammo) or immobile but not neutralized player is still hunted.
- Distress job spawns a Civilian derelict; the job is done on dock
  (`crates/nova_world_base/src/jobs.rs:892-927,1386-1411`). Nothing is towed.
- Docked assemblies fly as one body with one helm
  (`crates/nova_ship/src/sections/docking_section/mod.rs:19-26`). AI
  `ShipOrderDirective::Dock` flies only onto stations
  (`crates/nova_ship/src/flight/order.rs:114-129`). Both need a live port on
  the player.

## Open decisions (owner)
- What SOS does: a tug arrives and tows, or a fade to the nearest friendly
  station with a bill.
- Cost: credits, a cargo share, stance, or time.
- When SOS is offered: neutralized, no dock, no thrust, or any time.
- Whether a hostile can answer the call (risk) or only friendly
  civilizations.

## Delivery
1. An SOS action available to a stranded player, with its cost shown before
   confirm.
2. A rescue that resolves in the world and puts the player at a station with
   the ship kept.
3. Clear the neutralized state on rescue, or define why it stays.
4. Wiki and changelog.

## Verification
- Unit tests for the offer rule and the cost.
- ECS asserted example or nova-probe flow: a neutralized player calls SOS
  and ends docked at a station with the bill paid.
- A playable example.

## Done when
- No stranded state is a dead end: neutralized, dockless or immobile, the
  player can always call for help and continue.
