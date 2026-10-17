# Stance: implement the graph after the note is accepted

- STATUS: CLOSED
- PRIORITY: 47
- TAGS: v0.16.0, stance

## User facts
- Epic `20261012-100034`. Blocked on the research note. Stance table seeded by proximity; events flip stances with thresholds; allied neighbours follow, rivals warm, one hop only; escort sales and station purchase move to Ally rosters; the robbery refusal window becomes an edge flip.

## Agent decisions
- 2026-10-17: implemented the note's section 11 as accepted: `STANCE_NEIGHBOUR_RADIUS` 340 km (const-asserted against the 240 km lattice and 40 km jitter), the player table seeded by the `b"allegiance"` draw as before, edges 25 / 50 / 25 off a pair stream keyed by seed and ordered nodes with the designated rival, `OFFENCE_STEP` 3 in `OFFENCE_WINDOW_SECONDS` 3600, `MERIT_STEP` 3, one-hop propagation on an offence step only (Ally edge follows down, Enemy edge warms up), `PROVOKED_WINDOW_SECONDS` 900 on the player's 30 hp crossing (`ShipProvoked`), `ENEMY_DECAY_SECONDS` 7200, fine `FINE_CR` 1,000 to 10,000 cr by advancement paid at any admitting neighbour station, merchandise (owned station, fleet escort, neutralized hull) keeps its side.
- 2026-10-17: admission, the AI and the HUD read `Allegiance` alone; `nova_world_base::stance::sync_civilization_sides` writes each civilization's effective stance as the side of its live generated bodies every frame, so the `NeutralizedHull` refusal, `NeutralizedCivilizations`, `RobberyRule` and `PriceTable.robbery` died. Escort hire and station purchase read `Allegiance::Player` on the partner (`EscortHirePartner.allied`, `StationSaleType::NotAllied`).
- 2026-10-17: a merit is recorded at the hand-in site (`nova_interface` `jobs.rs` through `record_merit`), not by change detection on `Jobs`, which is unreliable across a Load and the cleanup frame.
- 2026-10-17 (owner): the fine moves credits with a new `CreditChangeCauseType::Settle`, not `Trade`, so a mod can tell a fine from a purchase.
- 2026-10-17: `nova_interface` now depends on `nova_world_base` (it was a dev-dependency): the Settle block needs the seeded neighbourhood and the fine, and the hand-in needs the seeded stance. No cycle: `nova_world_base` never reads the interface.

- 2026-10-17: the examples that hire escorts or buy a station (`system_station_hire`, `system_station_own`, `system_role_goal_empire`, `system_role_goal_yard`, `station_own_sandbox`) keep their non-Enemy choosers and warm the chosen station's civilization to Ally through `record_merit`, the player's own path (`MERIT_STEP` merits per step), before the gate. An Ally-only chooser found no station within reach on seed 10 (one Ally in 12 scout hops, never a pair), and a seed hunt or a wider hop budget would only hide the 25 % Ally share the design sets. `system_station_buy` keeps no warm step: parked hulls remain merchandise.
- 2026-10-17: the stance walk counts the pirate's charter as at least one civilization turned Enemy, not exactly one: an Ally of X seeded Neutral drops to Enemy by the approved one-hop propagation and counts too (seed 10: Fenhalka and Corannor). The walk stands off 2 km during the kills and pulls back one frame after the refused berth: a hull left at the berth takes contact damage, and an earlier run lost the player's docking port before the save.

- 2026-10-17 (review follow-up): the Settle block lists a neighbour by its table stance, not its effective one, so a provocation window never buys a fine without a step; the neighbourhood const assert now bounds the true face maximum `(L + 2J)^2 + 2 (2J)^2` (339.4 km) instead of `L^2 + 8 J^2`; `StanceStep::direction` died unused.
- 2026-10-17 (owner): every intact generated crew gets the 8 s arrival grace, whatever its side. The planner reads the seed alone, so a civilization the player turned Enemy spawns on the seed's side and flips the same frame; an Enemy-only grace skipped it. A Neutral or Ally crew provoked inside the window answers once it passes. Proof: `a_side_flip_inside_the_grace_holds_until_the_timer_runs_out` (nova_ship) and the generated-ship facts test (nova_world_base).

## Verification
- The epic's proof plus the sweep from the note as a permanent test.
- Unit: `civilizations.rs` sweep over 16 seeds and `[-4, 4]^3` (`every_living_civilization_with_a_living_neighbour_has_a_rival`, `an_edge_reads_the_same_from_both_ends_and_in_any_order`, `a_neighbourhood_never_leaves_the_block_round_the_node`); `stance.rs` `a_flip_moves_only_the_civilization_and_its_neighbours` and `the_fine_runs_between_its_ends_by_advancement`; `civilization.rs` table tests; `save/tests.rs` stance resume and future-stamp refusal; `goal.rs` outlaw.
- Live: `system_civilization_stance` (pirate, seed 10): dock admitted, three kills flip X Enemy, HUD `<NAME> REFUSES DOCKING`, berth refused, CHARTER at least 1 / 1, rival N Ally with every hull on the Player side, leave save and Load carry X / N / R, decay thaws X and the dock is admitted again.

## Done when
- The graph is in the save, admission and rosters read it, and the sweep passes.
