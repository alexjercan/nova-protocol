# Upgrades: add a section at a free attachment point

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 6, second rung: add a section at a free attachment point of the hull, station only, validated by post-build checks; an occupied point refuses and nothing changes.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Attachment points are implicit in generated layouts (`crates/nova_world_base/src/ship_layout.rs`); authored hulls have none declared. Bindings come from the hull's sections.
- Worker report 2026-10-14. Code: the rule is `crates/nova_ship/src/sections/section_add.rs` (`free_link_points`, `section_add_fit`, `add_build_fault`, `added_section_binds`, `added_section_id`, `plan_section_add`); the socket choice is `natural_link_point` in `link_points.rs`, shared with the editor's `snap.rs`. The spawn is the `add_design_section` observer in `crates/nova_scenario/src/objects/spaceship.rs`; the Ship pane list is `crates/nova_interface/src/ship/refit.rs`.
- Proof: `examples/systems/system_station_add.rs`. Base round: `block_workship` with no bindings; the add at the taken socket `HULL-1 positive_x` is refused (note `add: HULL-1 positive_x is taken`, design, credits and hold unchanged); the PDC is added through the pane at `HULL-1 negative_x` as `PDC-1` for 100 cr with an empty binding; it survives freeze and thaw. Generated round: an Armored hull (world seed 20261014, hull seed 3) gets a PDC at `HULL-1 positive_x` as `PDC-3` with the sibling turrets' `Mouse Left`; it survives freeze and thaw. Each add asserts the pose, no build fault on the live ship, fresh health, `OnAdded` fields and causes `Add`/`Add`. Run under Xvfb and lavapipe at 1920x1080: `autopilot: cycle complete, no panic (t=18.4s)`. Unit tests: six in `section_add.rs`.
- Ship side panel scrolls now (`Overflow::scroll_y`, wheel over the panel scrolls it and does not zoom the view; `scroll_ship_panel` in `crates/nova_interface/src/ship/app.rs`). This also fixes the swap task's clip gap for the rebuild and swap lists. The Map pane keeps a clipped panel.
- Gaps: the Add list sits under the Swap list, so on a hull section it is long (nine fitting parts) and needs a scroll. The editor's `snap.rs` still folds its own overlap refusal; a shared fit check with `section_add_fit` is follow-up debt. No roll or socket cycling at a station.

## Agent decision
Owner told the orchestrator to work the list without a stop at the gate. The agent chose the marked options and implemented them (2026-10-14).

- Attachment points. Survey: every section carries `LinkPoint` sockets with a position and an outward normal (`crates/nova_ship/src/sections/link_points.rs`); `snap_placement` already turns a target socket plus a part socket into a pose, and the editor builds with it (`crates/nova_editor/src/snap.rs`). That is enough data to place a part.
  - (A, chosen) A free attachment point is a socket of a live section that no section of the design (live or lost) mates. No new content. Consequence: base hulls, generated hulls and mod hulls all have points with no authoring, and a lost section's socket stays reserved for its rebuild.
  - (B) Authored attachment points per hull design, linted for overlap. Consequence: new content on nine base hulls and every mod hull; generated hulls would need the generator to emit them.
  - (C) Generated hulls only. Consequence: the player's base start hull could never grow.
- Placement. The player picks a free point on the selected section, then a part. The part mates the point with the socket that faces most nearly opposite (it keeps the orientation it was drawn in, the editor's default), roll 0. No roll or socket cycling at a station.
- Fit, in order: the point is not occupied; the part has a socket; its collider overlaps no design section it does not mate (`section_colliders_overlap`, the lint and editor rule); then the generator checks on the live ship, refused only for a fault the add makes: one socket graph (also catches a socket with two suitors), clear exit lanes (`placement_blocks_an_exit`), the thrust floor and the flight computer count. Role, size ceiling and symmetry are not checked, as for swap. A refusal changes nothing.
- Shared build check: `SwapBuildFaultType` and `SwapPlacedSection` become `RefitBuildFaultType` and `RefitPlacedSection`, used by swap and add.
- New section id: the greatest design id of its class followed by `+` (or `added_<class>` when the class is absent), with more `+` until unique. Codes count a class by id, so the new section takes the next code and no live code moves.
  - Rejected: `added_<n>`. Consequence: it sorts before most ids and would take a code a live section already holds.
- Binding:
  - (A, chosen) The new section takes the binding of the first live section of its kind (by id) that has one. With none: a weapon gets an empty binding that Rebind (B) sets; a thruster stays unbound so the flight computer drives it; a mining emitter takes the first of V, B, N, C that no emitter holds, and the add refuses when all four are held.
  - (B) Always unbound. Consequence: the Ship pane cannot rebind a section with no binding component, and a player mining emitter must have a key (the spawn panics).
  - (C) The kind's default key (LMB, F, R, Space). Consequence: a second copy of the editor's default table, and a bound drive leaves flight-computer control.
- Price: the new part's full recipe through `plan_recipe_payment`. Station only, Refit service.
- Event: `OnAdded` (`onadded`) beside `OnSwapped`: `id`, `section` (new code), `part`, `at` (code of the section it mounts on), `point` (socket id), `parts`, `price_cr`. Credit cause `Add`.
- UI: the Ship pane's Add list under the Swap list: point buttons for the selected section (first free point picked by default), then every catalogue part that fits there with its price or payment refusal and an Add button.

## Delivery gate
- Propose explicit attachment points on a hull design (content), lint for overlap, and the bind rule for a new section. Stop for the owner on whether base hulls get authored points or only generated ones.

## Verification
- Asserted example: add at a free point, post-build checks pass, binding exists; add at an occupied point refuses with nothing changed.

## Done when
- Add works in a live docked run on one base hull and one generated hull.
