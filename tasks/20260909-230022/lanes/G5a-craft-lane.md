# G5a craft lane

agent: agent-a7ba5208138c2fa8a
last_ts: 2026-09-09T20:45:48.145Z
stop_reason: end_turn
records: 228

## Dispatch prompt

```
You are the Craft lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/craft.md

Range under review: the three largest new loop producers added by commit `7fdd25222` ("Draft the v0.13.0 news post with its media and widgets"): `examples/screenshots/loop_hull_generate.rs` (760 new lines), `loop_helm_orders.rs` (395), `loop_goto_standoff.rs` (333).

Bundle, already built and PATH-SCOPED to those three files. Do NOT re-derive the range:
- /tmp/nightly-20260909/g5a/log.txt
- /tmp/nightly-20260909/g5a/stat.txt
- /tmp/nightly-20260909/g5a/diff.txt

1 488 new lines of example code landed in one commit. Judge: how much of it is duplicated between the three files and against the existing `examples/screenshots/shared/` helpers and the other loop producers already in the tree; whether anything here should have gone into `shared/` instead; whether `AppBuilder` is used as AGENTS.md requires; whether world units leak into any printed or logged figure that should be meters; and whether the docstrings say the reason rather than restating the next line.

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do NOT run these examples, a probe, or any benchmark.
- Never run the workspace test suite or Clippy. `nix develop --command cargo check --example <name>` is allowed and cheap.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

MAJOR - `examples/screenshots/loop_hull_generate.rs:1` (and `loop_helm_orders.rs:1`, `loop_goto_standoff.rs:1`) - the script half of each producer is gated with a per-item `#[cfg(feature = "debug")]` (48 + 26 + 15 = 89 attributes) instead of the one-gate-per-module shape the shared kit states as the rule.

- Failure scenario: at the reviewed commit, `loop_goto_standoff`'s `ARRIVAL_MARGIN` and `TRACE_ENV` and `loop_helm_orders`' `LEG_END`, `CROSSER_SPEED`, `SEND_AT_X` and `ORDER_KEY` were ungated consts read only by the gated half. `cargo check --workspace --all-targets` under default features failed on both files (and on four other producers). The producers were only ever run under `--features debug`, so nothing caught it until commit `9e69ac196` two commits later. The shape that produced it is untouched: 89 attributes is 89 chances for the next const to be gated on the wrong side, and the default-features build is the only thing that says so.
- `examples/screenshots/shared/ui_walk.rs:20-23` already states the rule and the reason: "everything here is script-only, so the whole module sits behind ONE gate at the `mod` instead of an attribute per item." `loop_hull_generate.rs:48-51` repeats that reasoning verbatim for its `mod ui_walk;` include, then spends 48 per-item gates on its own script half.
- Actionable: move each producer's script half into a `#[cfg(feature = "debug")] #[path = "shared/..."] mod script;` (or a per-producer sibling one level down) so one gate covers the consts, predicates and beats together. `catalog_matches_disk` only counts `.rs` directly under a category dir, so a sub-file is safe.
- Not BLOCKER now only because `9e69ac196` fixed the six acute breaks; `nix develop --command cargo check --example loop_hull_generate --example loop_helm_orders --example loop_goto_standoff` is green on the current tree. At the range under review it was a build path that fails.

MAJOR - `examples/screenshots/loop_helm_orders.rs:289` - `give_the_order` reimplements `nova_scenario`'s private `install_ship_order` and silently drops both of its guards.

- `install_ship_order` (`crates/nova_scenario/src/actions/ship.rs:841-861`) is `cancel_ship_order` -> insert-if-absent `ShipOrderReports` -> insert `(ShipHelmOrder::new(key, directive), ShipOrderHelmAuthority)`, behind `orderable_ship` (`ship.rs:878`). `give_the_order` is the same four statements in the same order, minus `orderable_ship`. Its own docstring calls this out ("the same three components the `PatrolShip` action installs ... minus the scenario that would report its lifecycle"), so the duplication is deliberate and undocumented as to why.
- Failure scenario: `orderable_ship` refuses a ship carrying `PlayerSpaceshipMarker` with a named error ("a helm order cannot share a helm with live input"), and resolves through `scoped_ship` (`ship.rs:1918`), the scenario-scoped lookup. The example resolves through its own `entity_by_id`, an unscoped sweep of every `EntityId` in the world. Change `GUNSHIP_ID`'s controller to `SpaceshipController::Player` while staging a variant of this loop and the order installs onto the player's helm and fights the live input for it, where the production path would have refused with a message naming the cause. The example also copied the `if !entity.contains::<ShipOrderReports>()` guard without the comment that explains it, so a future reader sees an unmotivated conditional insert.
- Actionable: make `install_ship_order` (or a thin `nova_scenario` wrapper over it) `pub` and export it through the crate prelude, and have `give_the_order` call it. The one behaviour the example genuinely wants that the action does not give it - opening the loop before the order lands - is a step ordering question, not a reason to re-derive the installer.
- Not BLOCKER: `SpaceshipController::AI` plus `Allegiance::Player` is not `PlayerSpaceshipMarker`, so nothing is wrong in the shipped set today.

MINOR - `examples/screenshots/loop_hull_generate.rs:169` - `POSE_EPSILON`, `pose_for_the_landing` (`:371`) and `the_landing_camera_is_posed` (`:408`) are copies of `shared/ui_walk.rs:64`, `:78` and `:151`, in a file that already includes that module.

- The three differ from the originals only in the two pose constants (`HULL_EYE`/`HULL_LOOK` vs `EDITOR_EYE`/`EDITOR_LOOK`). Bodies and docstrings are otherwise the same text, and `POSE_EPSILON` is the same `1e-2` with the same "compared against the camera Transform, so it is an engine world-unit figure" sentence.
- Failure scenario: `pose_editor_camera`'s docstring carries the reason the pin exists at all - "the editor camera is a free-fly WASD camera ... removing the component does not stop it: its private state survives ... every section the build placed landed on the face the ORIGINAL pose saw." If the free-fly rig changes and `ui_walk::pose_editor_camera` is updated to match, `loop_hull_generate`'s copy is not, the landing frame drifts off `HULL_EYE`, and `news-0130-hull-plan.png` re-captures at a bearing nobody chose while every beat still passes.
- Actionable: give `ui_walk` `pub fn pin_editor_camera(world: &mut World, eye: Meters3, look: Meters3)` and `pub fn the_editor_camera_is_posed(eye: Meters3) -> Arc<Predicate>`, make `POSE_EPSILON` their private business, and have `pose_editor_camera`/`the_build_camera_is_posed` be the `EDITOR_EYE` calls. One rule, two callers.

MINOR - `examples/screenshots/loop_hull_generate.rs:424` - `GenerateGestures` is a second gesture extension trait on `AutopilotPlugin<GameStates>`, and four of its six methods are walk-agnostic.

- `ui_walk::Gestures` (`shared/ui_walk.rs:271`) is the established home for editor-walk gestures, and its docstring states why the trait shape exists. `hold` (`:462`), `retype` (`:502`), `scroll_the_rail_to` (`:470`) and `tick` (`:538`) contain nothing specific to this walk: `retype` drives any `nova_ui` text field, `scroll_the_rail_to` drives the editor rail by its own node names (`RAIL_VIEWPORT`, `RAIL_BAR`), `tick` reads `nova_ui::prelude::Selected` on any `ListRow`, and `hold` is the capture-gated hold every loop producer wants. Only `press_the_zone_chip` and `press_generate` are this walk's.
- Failure scenario: the next editor walk that types a name into a field re-derives `retype`'s two non-obvious rules - press `End` before Backspacing because "the click lands the caret wherever the pixels fell", and wait on `not(the_field_holds_the_caret)` after Enter - and gets one of them wrong, producing a still with a half-cleared field at exit 0.
- Actionable: move `hold`, `retype`, `scroll_the_rail_to` and `tick` onto `ui_walk::Gestures` beside `click`/`arm`/`found`/`place`, along with the predicates they need (`the_field_reads`, `the_field_holds_the_caret`, `the_row_is_ticked`, `the_rail_shows`, `the_node_reads`). `nova_autopilot` is the wrong home for these - it depends on `bevy` alone by design, and reaching `TextFieldValue` would add a `nova_ui` edge for two predicates.

MINOR - `examples/screenshots/loop_hull_generate.rs:630` - the menu -> Sandbox -> editor -> Add > Ship prologue is a verbatim copy of `screenshot_editor.rs:155-180`, comments included.

- 21 lines, matched by clone detection across the two files: the same four steps, the same button names, and the same two explanatory comments ("reached the way a player reaches it. `click` waits for the button to lay out" and "Through the top bar, because that is where Add lives"). `screenshot_planet_editor.rs:75` carries a third partial copy.
- Failure scenario: rename `Sandbox Button` or move Add out of the top bar and three producers need the same edit; miss one and it fails at `STEP_DEADLINE_SECS` on "reach the editor" with no indication that the other two were already fixed.
- Actionable: add `fn enter_a_blank_ship(self) -> Self` to `ui_walk::Gestures` and call it from all three.

MINOR - `examples/screenshots/loop_goto_standoff.rs:169` and `loop_helm_orders.rs:148` - `refuse_broken` is copied a sixth and seventh time.

- The two new bodies are byte-identical to each other once the panic-message prefix is removed, and differ from `examples/playable/first_shift_ships.rs:160` only in that string. `first_shift_map.rs`, `shape_bench.rs`, `wfc_arena.rs` and `block_bench.rs` carry further variants of the same 25 lines.
- Failure scenario: `lint_scenario`'s signature gains a parameter (it has already grown one - the channel-id set is threaded through all seven copies) and seven call sites need the same edit; a producer whose copy is updated to pass `HashSet::new()` for channels instead of `build_channels()` silently stops catching an unresolved dialogue cue and the set loads with a broken channel.
- Actionable: one `pub fn refuse_broken(what: &str, scenario: &ScenarioConfig, sections: &GameSections, ships: &GameShips)` in `shared/kit.rs`, which both new producers can `#[path]`-include without pulling in `hollow.rs`/`ring.rs`.

MINOR - `examples/screenshots/loop_goto_standoff.rs:217` and `loop_helm_orders.rs:207` - `entity_by_id` is copied a fourth and fifth time, when the identical function is already `pub` in two shared modules.

- `shared/hollow.rs:648` and `shared/ring.rs:528` both export `pub fn entity_by_id(world: &mut World, id: &str) -> Option<Entity>` with the same body. The new files could not reach either without including the whole module (and `hollow`/`ring` each pull in `kit.rs`, which the module docs warn against double-including), so the copy is understandable - but the answer is to move the helper down, not to add copies.
- Failure scenario: `helm_orders` also needed a `&World` form and wrote `entity_by_id_ref` (`:216`) with a different implementation (`iter_entities` scan vs `query`), so the same file now resolves an id two ways; a change to `EntityId` scoping applied to one and not the other makes a beat advance on one entity and a hook act on another.
- Actionable: move `entity_by_id` and a `&World` twin into `shared/kit.rs`, and have `hollow.rs`/`ring.rs` re-export them rather than define them.

MINOR - `examples/screenshots/loop_hull_generate.rs:224` and `:237` - `named_node` re-derives a name lookup that `nova_autopilot::input` already owns with stricter rules, and `zone_chip` reaches into `nova_editor`'s child order.

- `named_node` scans `(Entity, &Name)` raw. `ui_node_rect` (`crates/nova_autopilot/src/input.rs:357`) does the same resolve but filters on `InheritedVisibility`, rejects a node that has not been through a layout pass, and warns when two laid-out nodes share a name - its docstring records why each of those was added ("a settings-panel screenshot shipped as the bare main menu at exit 0"). `named_node` has none of them, and its docstring ("The first UI node called `name`.") restates the body rather than saying why the crate's rules are being skipped.
- `zone_chip` takes `Children::last()` because `nova_editor`'s `PartZoneChip` marker (`crates/nova_editor/src/config.rs:465`) is `pub(crate)` and unreachable. Failure scenario: add a trailing badge to `part_row`'s `children![...]` (`crates/nova_editor/src/ui/rail.rs:210-241`) - a count, a spacer, a warning glyph - and `zone_chip` returns it, `zone_chip_label` reads its text, `the_zone_chip_reads(ZONED_ROW, "bow")` never matches, and the run dies at `BEAT_DEADLINE_SECS` on "cycle the turret's zone 1/6" with nothing pointing at the row layout. `nova_editor` compiles and its own tests pass.
- Actionable: split `ui_node_rect` into `pub fn named_node(world, name) -> Option<Entity>` (with the duplicate-name warning) plus `pub fn ui_node_rect_of(world, entity) -> Option<Rect>` (with `is_a_target`), keep `ui_node_rect(world, name)` as their composition, and use both from the walk - that also removes the hand-rolled rect-and-guard in `zone_chip_centre` (`:245`). Export `PartZoneChip` through a `nova_editor` prelude so the chip is asked for by identity rather than by position.

Checked:

- Full diff of all three files from `/tmp/nightly-20260909/g5a/diff.txt`, and the current tree state of all three (`loop_goto_standoff.rs` and `loop_helm_orders.rs` have moved since the range - `a5f7df1d6` and `9e69ac196` - so every anchor above is a tree line number and every finding was re-confirmed against the tree).
- Clone detection (normalised 8-line windows) of the three files against all 49 files under `examples/`, which produced the duplication figures quoted: 52 shared lines between `loop_goto_standoff` and `loop_helm_orders`, ~25 each against `first_shift_ships`/`first_shift_map`, 21 between `loop_hull_generate` and `screenshot_editor`.
- All eight `examples/screenshots/shared/*.rs` helpers, for what should have gone in: `kit.rs`, `ui_walk.rs`, `hollow.rs`, `ring.rs` exports read in full or by export list.
- `AppBuilder`: all three comply. `loop_goto_standoff:59` and `loop_helm_orders:52` use `AppBuilder::new().with_game_plugins(..)`; `loop_hull_generate` uses `editor_app(true, None)`, which is `AppBuilder` (`crates/nova_core/src/lib.rs:82-88`).
- World units in printed and logged figures: no leak found. `report_the_park_points` (`loop_goto_standoff.rs:410`) converts the avian position and `HullRadius` with `Meters::from_engine` before formatting and prints `m`; `loop_helm_orders` and `loop_hull_generate` print no distances. Every `to_engine`/`from_engine` crossing carries a local "Engine boundary" note naming what it is crossing for (`FlightArrivalStandoff` is world units per `crates/nova_ship/src/flight/state.rs:70`; `LinearVelocity`, `avian3d::Position`, `ShipOrderDirective::Patrol` waypoints, `HULL_EYE` vs camera `Transform` likewise). `loop_hull_generate`'s two engine-unit `f32`s (`POSE_EPSILON:169`) say so in their docstrings. `HULL_EYE`'s docstring arithmetic checks out: 230 m range, 21 degrees elevation, 12 m screen-right push, and 13x5x11 cells = 130x50x110 m at 10 m per cell.
- Docstrings: they state the reason, not the next line, with the one exception folded into the last finding. Body comments are uniformly why-comments; I found no comment restating its next line and no comment left stale by the range.
- `#[allow(...)]`: none in the three files. No new workspace lint. No compatibility shim or option-to-spare-a-caller.
- `nix develop --command cargo check --example loop_hull_generate --example loop_helm_orders --example loop_goto_standoff` under default features: clean.

Not checked:

- `--features debug` compile of the three producers. The default-features check was warm; the `debug` build was not, and the range's `debug`-only half is where every gate above lives. This is the half CI covers and I did not re-run it.
- Whether any of the three actually captures. I do not hold the measurement slot: no example, probe or benchmark was run, so nothing here says the loops record, that `news-0130-hull-generate.webm` frames as its docstrings claim, or that the `ZONE_CYCLE` order matches `nova_editor`'s live `next_zone`.
- Frame cost, `LoopProfile` encode settings, test coverage, changelog and web-side wiring - other lanes.
- The 6 pre-existing `refuse_broken` copies and the 2 pre-existing `entity_by_id` copies as findings in their own right; they are outside the range and appear above only as evidence.
- `LoopProfile`'s docstring (`crates/nova_autopilot/src/loops.rs:126-128`) says a caller "selects another framing by building the whole struct ... rather than a default plus edits", and `generate_loop_profile` (`loop_hull_generate.rs:214`) uses `..fleet`. I did not raise it: both pre-existing callers (`wfc_arena.rs:2401`, `stress_hull_collapse.rs:301`) do the same, so the stale item is the `LoopProfile` docstring, which is outside the range.
