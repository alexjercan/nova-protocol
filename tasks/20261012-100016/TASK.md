# Fleet: hire an escort at a station

- STATUS: CLOSED
- PRIORITY: 73
- TAGS: v0.16.0, wizard

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

- Orchestrator close 2026-10-14: landed in 7d85c0993..98f94634d. 1 to 3 armed escorts per station are listed in the existing catalogue (`HullOfferType::Hire { slot }`), gated by `Refit`, at the hull price times 3/2. The fleet has two slots (starboard, then port), is saved in `SavedPlayer.fleet` (format 8) and survives world replacement. A neutralized escort is gone for good and leaves a normal wreck. Live-run fixes: the streaming reset no longer deletes thawed fleet escorts, and `follow_waypoint` routes an escort round its leader. Proof: `examples/systems/system_station_hire.rs` (hire, cross cells, save, Load, follow, kill, gone after a second save and Load). The orchestrator reviewed the crossed frame (player and escort side by side) and re-ran `content lint`: 0 errors. `SYSTEMS_INVARIANTS` 535. Gaps: escorts avoid only their leader, not stations (the walk jumps 2 km clear after undock); other station examples may hide an undock collision through the station; no wages and no orders; five older `[Unreleased]` entries exceed 200 characters.

## Owner feedback (2026-10-14, reopens the task)
- A hired escort is put in a position around the player instead of flying. It must follow the player with its AI flight controller (steering, thrust, matching speed), not orbit or snap to slots.

- Orchestrator close 2026-10-14 (owner feedback): landed in 5ab1c02eb..044fa3f04. The escort was never moved directly; its slot was fixed to the leader's nose, so a turn swung the slot round the player and the escort chased it (it reached 176 m/s and overshot to 870 m in a 90 degree turn). Now the slot follows the leader's travel direction. The escort holds the leader's velocity plus a closing speed it can brake from, plans its leg round the leader, hulls and bodies (`plan_leg`), and holds position when the leader is under 10 m/s. `follow_waypoint` is deleted. Proof: `an_escort_flies_formation_on_its_leader_under_its_own_drive` in follow.rs, and claim 3 of system_station_hire (15 m off slot after a 90 degree turn at 50 m/s, no step unexplained by velocity). SYSTEMS_INVARIANTS 546. Orchestrator re-ran `content lint`: 0 errors; reviewed the after turn frames. Values for owner review: slot 200 m to the side and 100 m back, hold 150-400 m, clearance size plus 100 m, 30 m in-position tolerance. Gaps: two escorts and a reversing leader not run; in the live turn the inside escort came to 81 m of the hull (inside the 100 m clearance, no contact).

- Owner feedback 2026-10-14, second round (reopens the task):
  - hiring several escorts spawns them on top of each other;
  - escorts still seem to gain speed or inertia from the player when the player starts to move. An escort must gain velocity only from its own thrust and simply follow.

- Owner feedback 2026-10-14, third round: escorts still seem to gain velocity the moment the player starts moving, "like they get a relative force applied to them". Investigate further under real manual input.

- Owner response 2026-10-14 to the RCS finding (755849592): the RCS push was acceptable behaviour; the owner could not HEAR it. Decisions:
  - escorts may still use RCS (fine positioning near the slot, dodges), but when the leader flies off they must turn and fly on the main drive, not slide on RCS;
  - make RCS use more obvious (audio, and visuals if missing) for every ship, not only escorts;
  - the player must be able to dock with an escort, so an escort must not dodge or clear off from a player who is approaching to dock.

## Delivery gate
- A `fleet` field in `SavedPlayer`; an escort is a generated hull with `Allegiance::Player` and a follow order; it crosses sectors with the player (not frozen in the old cell). Stop for the owner on the escort's death rule (gone for good vs. rebuyable).

## Verification
- Asserted example: hire, cross a sector, save, load, the escort is beside the player and still follows.

## Done when
- Hire works in a live run and the escort survives a crossing and a reload.

## Orchestrator close (2026-10-14)
- Owner feedback rounds landed: spawn clearance (eb0af0561), drive-first follow and lag (3a173fe55, 755849592), trim zone, dock hold and stop order while docked (910f933c5), audible and visible RCS (37b148b7c).
- Playable proof: `examples/playable/escort_sandbox.rs` (661adf6af) with the F3 overlay (slots, trim zone, velocity and RCS arrows, dock-hold radius). Smoke PASS, hand run stays open.
- Orchestrator re-ran `content lint` on the working tree: 0 errors, 0 warnings, 0 findings.
- Owner review values still open: 200 m side and 100 m back spacing, 50 m spawn clearance, 2 s lag, 20 m/s closing cap, 60 m trim zone, 400 m dock hold, RCS volume 0.12.
