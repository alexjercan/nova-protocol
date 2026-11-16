# SOS: a stranded ship can call for rescue and continue

- STATUS: CLOSED
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

## Open decisions (owner) - resolved by the Decisions below
- What SOS does: a tug arrives and tows, or a fade to the nearest friendly
  station with a bill.
- Cost: credits, a cargo share, stance, or time.
- When SOS is offered: neutralized, no dock, no thrust, or any time.
- Whether a hostile can answer the call (risk) or only friendly
  civilizations.

## Decisions (owner said start 2026-11-16; agent calls, stated)
- What SOS does: a timed rescue call on the gameplay clock (60 s; TAB and
  pause hold it, Cancel is free). At the end the player ship and its fleet
  move to the chosen station at rest and dock there. No tow, no fade screen.
- When it is offered: only to an undocked open-world player with no station
  berth under way who is stranded: no working thruster, no working flight
  computer, or no free live docking port. Not neutralized: a ship that lost
  only its guns flies and docks (review, 2026-11-16). Not any time, so it is
  not free fast travel. A reason the chain cannot reach (a live but inactive
  section, with none lost) refuses at the click with a pane note.
- Who answers: the nearest living station whose civilization is Ally or
  Neutral to the player, picked at the click. Hostile stations never answer.
  No station answers: refuse at the click with a pane note.
- Cost: 200 cr fee plus the rebuild price of each section the rescue
  restores; hold parts are used first. Credits stop at zero (`ShipCredits`
  is u32), so a broke player still continues. New credit cause `Rescue`.
- When it is paid: at the answer, in one step (review, 2026-11-16). The
  answer first checks the station would admit the dock (alive, not Enemy,
  not hostile to the ship); a refusal takes nothing. Then it takes the bill,
  rebuilds the chain and moves the ship. A berth that then gives up leaves
  the paid, fixed ship beside the station to dock by itself. A ship that
  docks before the answer ends the call for free.
- The rescue rebuilds the shortest chain of lost sections to a live port,
  thruster and flight computer, so the ship can dock and fly. Weapons are not
  restored.
- Neutralized becomes a live verdict for the player: it clears when the rule
  stops holding (any repair path), and AI can target the player again.
  `DefeatedMarker` stays. AI ships keep the one-way marker.
- The call is saved, like repair jobs (owner correction, 2026-11-16): a
  Load resumes a running call. The berth after the answer is not saved: a
  save between the answer and the dock loads the paid, fixed ship beside the
  station, undocked. Map: `MAP.md`.

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
- No stranded state is a dead end: dockless or immobile, the player can
  always call for help and continue; a neutralized ship that still flies
  docks for repair.

## Delivery record (2026-11-16)
- Landed: offer rule and bill (`crates/nova_gameplay/src/rescue.rs`), timed
  call (`crates/nova_ship/src/sections/rescue_call.rs`), station search,
  answer and berth (`crates/nova_world_base/src/rescue.rs`, `start.rs`:
  `StationBerth` replaces `StartBerthMarker`), Ship pane Distress block,
  HUD chip and notices, live neutralized verdict for the player, save and
  resume of repair jobs and the call, `examples/playable/sos_rescue_sandbox.rs`.
- Review (independent, read-only) found 9 defects; all fixed: bill moved to
  the answer, docked guard and cancel on dock, guns-only loss is not
  stranded, berth skips held and inactive ports, answer ordered after repair
  jobs, escorts to clear slots, chase camera shifted at the jump, berth
  budget counts only game-time frames, one credit format.
- Verified: targeted unit and ECS tests in nova_gameplay, nova_ship,
  nova_world_base, nova_scenario (save round trip); live under Xvfb and
  lavapipe: sos_rescue_sandbox and field_repair_sandbox walks PASS; real game
  New Game, Exit, relaunch, Continue: a plate job (1 of 3 plates) and an SOS
  call (13 s in, nothing charged) both resumed and finished; rescue paid
  1,400 cr, rebuilt DOCK-1, THR-2, CTL-1, docked, neutralized cleared.
  Damage for those runs came from a save edit at the main menu: no cheat
  damages a section.
- Not verified live: escort placement, the camera shift, the refused and
  no-berth endings (unit tests only). Workspace tests and clippy not run.
