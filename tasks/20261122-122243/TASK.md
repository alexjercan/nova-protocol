# Canisters: pull by distance with a larger range, label all in range

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.19.0, mining, hud

## User facts
- Owner 2026-11-22: "increase the 'sensitivity' of how far the 'pull' on
  canister can go, it's not only when the cargo bay door is open but more
  like based on a distance and it should be a bit larger, also when
  canisters are in range we should show what is in them, like we do now,
  but now the range is really short and probably also show the gizmo line
  from all canisters that are in range, not only the closest one".
- Follow-up to the closed mining task "canisters drift to the open intake".

## Delivery
- The pull works by distance alone, with a larger range.
- Contents labels and lines for every canister in range, not only the
  closest.
- Build running (2026-11-22).

## Verification
- Asserted pull by distance; frames with several canisters labelled.

## Done when
- Canisters in the larger range pull in and all show contents and lines,
  and the owner has played it.

## Build (2026-11-22)
Map (before):
- Pull gated on a fully open door (`pull_canisters_to_open_intakes`);
  door by `detection_range`; base 40 m at 5 m/s, wide 60 m at 8 m/s.
- Tags: `CARGO_TAG_RANGE` 100 m from the ship centre. Lines:
  `CARGO_PICKUP_SIGHT_RANGE` 200 m, one pair only (lock, else nearest).
- Mined canisters are born about 109 m from the tug's dorsal intake
  face (beam reach 100 m), so the 100 m tag range missed them.

Decisions:
- One reach: `pull_range` drives pull, door, tag and line.
  `detection_range` deleted **(breaking)**; `deny_unknown_fields` on
  `CargoIntakeSectionConfig` makes a stale field fail the parse.
- Base 120 m at 8 m/s (about 15 s from the edge), wide 160 m at 10 m/s.
  Speed raised because 120 m at 5 m/s is 24 s.
- The pull has no door gate. The door still opens for an in-reach
  canister the hold has room for; a jettisoned canister no longer holds
  it open.
- HUD reach is geometric (`CargoPickupPair::in_reach`): in front of the
  face and within `pull_range`. A full-hold canister in reach keeps its
  tag and line; one behind the face, or near a ship with no intake, gets
  no tag now.

Change:
- `crates/nova_ship/src/sections/cargo_intake_section.rs:111` config,
  `:236` `in_reach`, `:460` `in_pull_reach`, `:677` door,
  `:815` `pull_canisters_to_intakes`.
- `crates/nova_hud/src/pickup_sight.rs:188` `select_lines` (one line per
  canister in reach, nearer face), `:148` `draw_sight`.
- `crates/nova_hud/src/cargo_canister_chips.rs:161` tags from in-reach
  pairs; `CARGO_TAG_RANGE` removed.
- `crates/nova_scenario/src/lint/ship.rs:744` `capture_gap < pull_range`.
- Builder `crates/nova_authoring/src/base_content/sections/cargo_intake.rs:181,207`;
  lesson text; gallery focus card; fixtures; `mine_and_sell` frames the
  pickup shot round every canister in reach (`frame_reach`).

Proof:
- nova_ship `cargo_intake` 23 pass, incl.
  `a_canister_at_the_pull_range_edge_is_pulled_before_the_door_is_fully_open`,
  `a_canister_past_the_pull_range_or_behind_the_face_is_not_pulled`.
- nova_hud 11 pass, incl.
  `every_canister_in_reach_draws_a_line_and_none_out_of_reach`,
  `a_label_shows_for_every_canister_in_reach_and_none_out_of_reach`.
- Lint, authoring lint_walk and world_base catalog tests pass.
- `mine_and_sell` live (lavapipe): 2 canisters in reach at the pickup
  frame, both tagged and lined; both taken 18.17 s and 17.00 s after
  birth (2026-11-21 median for the tracked one: 51.82 s); 9 ore sold;
  cycle complete, no panic.
- Frame: `~/Pictures/nova-canister-pull/pickup-two-canisters-in-reach.png`.

Stale media: `assets/loops/loop-section-cargo-intake.webm`,
`wiki-section-cargo-intake.png`, `mine-sell-taken.png`, the
`flight_cargo_pickup` lesson loop, start-role sandbox ring shots.

Not verified: hand play; wide intake live; AI intakes pulling at 120 m
near stations; overlapping tags for canisters side by side.

## Owner check (2026-11-22)
- Played and approved: "works".
