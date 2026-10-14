# Fleet: hire an escort at a station

- STATUS: OPEN
- PRIORITY: 73
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 13 (2026-10-12): the fleet half of D13 lands right after Stage 2: escorts for sale at any living station until the stance graph lands, then at Ally rosters; the fleet list lives in the player save.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- AI follow and patrol behaviours exist (`crates/nova_ship/src/input/ai/`); `FrozenAI` keeps behavior, patrol, leash (`crates/nova_ship/src/input/ai/frozen.rs:27-34`). Allies are `Allegiance::Player` (`crates/nova_gameplay/src/relations.rs:37-44`).

## Agent decision
Owner told the orchestrator to work the list without a stop at the gate. The agent chose the marked options and implemented them (2026-10-14).

- Death rule:
  - (A, chosen) Gone for good. A neutralized escort leaves the fleet at once: it loses its fleet marker and its follow order, so its cell adopts it and freezes it as a normal wreck, with its hold, like any neutralized ship. A destroyed one is simply gone. Consequence: no rebuy record, no new save field; `OnNeutralized` and wreck cargo work as for any ship.
  - (B) Rebuyable at a station. Consequence: a per-escort record that outlives the body (design, price, where it can be rebought) in the save and a second list in the Ship pane.
- Gate: the existing `Refit` service; no content change. A station with no escort offer shows no list, so authored stations and dead stations hire none.
  - (B) A new `Hire` service. Consequence: a content change to every generated station design and the station lint, for a gate that says the same as Refit until the stance graph lands.
- Roster: generated in `crates/nova_world_base/src/station_hulls.rs` beside the hulls for sale, one catalogue per living generated station. 1 to 3 escorts (seeded), only armed roles (scavenger, armored) the civilization's advancement can build; a station whose civilization builds no armed role hires none. Each is `generate_ship` at the civilization's advancement, keyed by the world seed, the station id and the slot, so the hulls for sale do not move. Name: `<Civilization> <role> escort <n>`.
  - Storage (A, chosen): each catalogue entry carries `terms: HullOfferType` (`Sale` or `Hire { slot }`), so escorts ride the station's existing `StationHullCatalogue`, `SectorStation.hulls`, `FrozenStation.hulls`, the manifest check and the canonical description. Consequence: no new station field; hulls for sale keep the first indexes.
  - (B) A separate `StationEscortRoster` and `FrozenStation.escorts`. Consequence: a second copy of every hull-catalogue seam (spawn, thaw, freeze, validate, describe).
- Hire consumes the offer: the escort leaves the station's catalogue and the frozen station keeps the shorter list. The escort's id is `escort_id`: the station's cell, the stem `<station stem>_<slot>_escort` and the roster slot as index, so it is unique for the world and does not move when an earlier escort is hired.
- Price: the sum of the hull's section recipes at ask, times the hire factor 3/2 (`ESCORT_HIRE_FACTOR` in `crates/nova_ship/src/sections/escort_hire.rs`), for the crew. Player `balance - price`, station `balance + price`. Refused when not docked, no Refit, the fleet is full, the player cannot pay, or a balance overflows.
- Fleet cap: 2, the count of `FLEET_SIDES` (starboard, then port of the player).
- On hire: the escort spawns undocked beside the player, on the side away from the station, at rest relative to the station, with `Allegiance::Player`, no hold and no credits, an AI controller with a 3 km leash, and a follow order (`AIFollow`) on the player.
- Follow AI: `crates/nova_ship/src/input/ai/follow.rs`. An `AIFollow { leader, side }` ship that is not fighting holds a slot beside its leader (both hull arms plus a 200 m gap along `side` in the leader's frame) through `MatchVelocity`, matching the leader's velocity and facing. Its leash centre rides the leader, so it engages hostiles near the leader and breaks off past 3 km. While the leader is docked it holds where it is. A leader gone ends the order. A slot across the leader is reached round it (`follow_waypoint`): a live run had the first escort, spawned away from the station, fly through the player to its starboard slot on the station side. It avoids no other body.
- Streaming: `PlayerFleetMarker` (`nova_gameplay`). The streamed world never adopts or freezes a fleet escort, and the sector snapshot leaves it to the save, as it leaves the observer (`crates/nova_world/src/frozen.rs`). `clear_sector_work` also leaves the fleet: a Load thaws it with the player before the world arms, and the arming swept it as a stray top-level body (a live run panicked on it). This is the rule for the handler-spawned-ship debt: a body travels with the observer only when it is in the fleet; every other unowned persistent body still refuses the save.
- Save: `SavedPlayer.fleet: Vec<FleetEscort>`. `FleetEscort` (`crates/nova_scenario/src/objects/fleet.rs`) is the escort's id, name, pose, motion, slot side and `FrozenShip`. The ids, items, holds and the cap are checked like every saved ship. A Load hands the fleet to `ResumedSpaceship`, and the player spawn thaws each escort beside the player with its follow order on the new player entity. Save format 8 stays (unreleased); its changelog line names the fleet.
- Event: `OnEscortHired` (`onescorthired`): `id`, `type_name` (the player), `escort` (the new ship's id), `hull`, `price_cr`. Credit cause `EscortHire` on both parties.
- UI: the Ship pane's side panel, an "Escorts for hire" list under "Ships for sale". Each row: name, length and section count, a class summary, price, and a Hire button or the refusal (a full fleet names the cap).

- Agent result 2026-10-14: implemented as decided, in 7d85c0993, 8c808ad71, c3131ba4d, 39e7fd1ba, 2d25677a7, a45eb2ec8, 6ea4d24e9 and the docs commit. Proof: `examples/systems/system_station_hire.rs` (seed 10, offer 5 'Fenhalka armored escort 1' for 16,320 cr; 5 roster invariants, `SYSTEMS_INVARIANTS` 535) passed under lavapipe: hire, a jump three cells out, the escort back on its slot, save, Load, the escort on its slot after the player jumps on, then two flight computers killed, the wreck in cell (5,0,0), an empty saved fleet and no fleet after Load. Unit tests: the roster (`station_hulls.rs`), the price and slots (`escort_hire.rs`), the save round trip (`save/tests.rs`) and the detour (`follow.rs`). Found on the live runs: a Load swept the thawed escort as a stray body (fixed in 2d25677a7); the first escort flew through the player to a slot on the station side (fixed in a45eb2ec8); the walk's berth left the mouse rig on the spawn heading, so the hull turned back through the station on undock (fixed in the walk). The follow order avoids only its leader, so an escort whose slot lies across a station still flies into it; the walk flies the player 2 km clear first. The escort has no crew wage and no orders.

## Delivery gate
- A `fleet` field in `SavedPlayer`; an escort is a generated hull with `Allegiance::Player` and a follow order; it crosses sectors with the player (not frozen in the old cell). Stop for the owner on the escort's death rule (gone for good vs. rebuyable).

## Verification
- Asserted example: hire, cross a sector, save, load, the escort is beside the player and still follows.

## Done when
- Hire works in a live run and the escort survives a crossing and a reload.
