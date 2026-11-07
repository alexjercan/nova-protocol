# Independent review findings

Two read-only reviews of `51397ef7a..f523527f8`. Reading only; nothing was run.
Treat each as a claim to reproduce, not a fact. Paths are under
`crates/nova_editor/src/` unless the path says otherwise.

## Persistence and undo

P1 HIGH. Import overwrite destroys the old mod first. `project/package.rs:757-779`:
`remove_mod` and `remove_index_record` run before `install_local`. A failed
install leaves nothing. Stage in a sibling directory; swap after success.
Status: FIXED `a_failed_import_over_keeps_the_old_mod`, `a_replace_that_cannot_land_leaves_the_mod_and_its_record`, `a_replace_after_a_kill_mid_swap_brings_the_old_mod_back_first`.

P2 HIGH. Save is not atomic and writes the manifest first.
`crates/nova_assets/src/mod_cache.rs:744-764`, `project/store.rs:274-281`. Crash or
ENOSPC leaves a truncated file or a manifest naming a missing file. Use
`storage::write_atomic`, manifest last.
Status: FIXED `a_failed_save_keeps_the_old_manifest`, `a_stored_file_is_replaced_in_one_step`.

P3 HIGH. A copy of an old `editor_*` save loses script handlers.
`project/store.rs:104-109,138-145,161-180`, `following_the_objects` in
`scenario.rs` (~1300), `standing_ship` `scenario.rs:1051`. Whole-match leaf rewrite
also hits spawn `base.id` and script refs; `is_old_save` stays true after
migration, so `editor_x.ship_1` becomes `<to>.editor_x.ship_1`. Handlers no longer
match the spawn and are dropped with no message. Rewrite only Prototype refs and
design ids, only for designs still bare.
Status: FIXED `a_copy_of_an_old_save_keeps_its_spawn_ids_and_its_handlers`, `a_copy_of_a_resaved_old_layout_moves_its_designs_once`.

P4 HIGH. One edit to an imported hand-written scenario drops authored ship data.
`bundle.rs:296,342-371`, `scenario.rs:1056-1062,1080-1085`, `station.rs:82-120`.
`LiftedShip` keeps no stock, credits, `lootable`, `capabilities`,
`initial_velocity`, `section_patches`. Station services, builds, stock are rebuilt
from constants. `Prototype { id, .. }` takes the node id from the design, so the
spawn id is lost, two spawns of a design clash, and handlers are dropped. Keep any
spawn with non-default fields, or an id unlike the design tail, as a raw object.
Status: FIXED `one_edit_to_a_hand_written_scenario_writes_only_that_edit`, `moving_a_hand_written_spawn_writes_only_its_position`. Reproduced over `webmods/gauntlet`: 1124 lines became 1127 on one description edit.
Also found and fixed: a raw spawn dropped the `bounds` handler and every `$other_id` handler.
Limit: all three real mods mix spawns with other actions in `OnStart`, or set `once`, so they
have no layout handler. Those spawns are Events blocks, not stage objects, as before this
branch. The raw route is proved on the same real spawns moved into a layout (`with_a_layout`).

P5 MED. Project verbs skip the incomplete-block guard. `project/verbs.rs:154-178,
346-354`, `event.rs:1642-1654`. `project_step` lowers without a gap check, so an
unfinished block vanishes. Call `refused_by_gaps` in `project_step`.
Status: FIXED `a_project_step_stops_on_an_unfinished_block`.

P6 MED. Mesh import overwrites a saved file outside history. `asset/verbs.rs:477-489,
541`. `resource_name` checks declared resources only. Import, save, undo, import a
different same-named glb, Discard: saved manifest points at the wrong mesh. Check
disk through `store::files_of`, compare bytes.
Status: FIXED `an_import_never_overwrites_a_file_a_save_may_name`.

P7 MED. Copies leave map keys on the old id and report success.
`project/store.rs:161-180`, `project/verbs.rs:577-605`. `map_string_leaves` skips
keys, so a recipe key still names the source mod's item. Rewrite keys or refuse.
Status: FIXED `a_copy_of_a_mod_moves_the_recipe_keys_with_its_items`.

P8 MED. Package and copy ship every file in the folder. `project/package.rs:828-841`,
`project/store.rs:294-301`. Orphan meshes leak in. Use manifest-declared files only.
Status: FIXED `a_package_and_a_copy_leave_out_files_the_manifest_does_not_name`.

P9 MED. Import checks the exact id. `project/package.rs:798`, `project/ids.rs:79,90,
113`, `crates/nova_assets/src/merge.rs:1158`. `editor_a_b` re-imports as `editor_a-b`
beside the original; both scenario namespaces are `a_b__*` and overlay last-wins.
Use `is_taken`, ask before overwriting.
Status: FIXED `an_import_lands_on_the_mod_of_the_same_published_id`.

P10 MED. Import accepts reserved names. `project/package.rs:681-689,701-704,742-744`,
`project/ids.rs:63-73`. `base` makes `editor_base`, and later imports rewrite
`dep://base/` to it. Apply `project_id` rules; treat shipped ids as installed.
Status: FIXED `a_package_named_for_the_game_is_refused`, `a_ref_into_the_game_stays_on_the_game_through_an_import`.

P11 LOW. Typed ids keep `-` in the namespace. `asset/verbs.rs:274-278`. Normalise
as `scenario_id` does at `project/ids.rs:113`.
Status: FIXED `a_typed_id_takes_the_spelling_mod_content_ids_have`.

P12 LOW. Rename crosses content kinds. `asset/verbs.rs:185,299,339-365`. `mint_id`
is free per kind, so two kinds can share an id and a rename moves both. Mint and
check across kinds, or scope the rename.
Status: FIXED `a_new_item_takes_no_id_another_kind_holds`, `a_rename_refuses_an_id_two_kinds_share`, `a_rename_refuses_an_id_another_kind_holds`.

## Canvas and input

C1 CRASH. Del during a held canvas press or drag. `placement.rs:434`,
`ui/canvas/mod.rs:524` have no guard. Panics: `ui/canvas/drag.rs:666-668` to
`event.rs:1911`; `ui/canvas/drag.rs:605` to `event/blocks.rs:767`; `.expect` in
`next_ordinal` `event/blocks.rs:377`. Refuse delete and clipboard keys during a
candidate or held drag; `land` and release re-check entities.
Status: FIXED `del_is_refused_while_a_block_is_pressed`, `the_clipboard_keys_are_refused_while_a_block_is_pressed`, `a_block_gone_from_under_the_hand_refuses_its_drop`, `a_slot_gone_from_under_the_hand_refuses_its_drop`, `a_click_released_on_a_dead_block_selects_nothing`.

C2 SILENT. Value retyped to an operator keeps counter 0. `event.rs:1875`,
`event/blocks.rs:375,510`. Operands get ordinals 1 and 2, `NextChildOrdinal` stays
0, a dropped reporter takes ordinal 1, and `0 - x` lowers as `x - 0`. Set the
counter in `retype_expression`.
Status: FIXED `a_reporter_dropped_on_a_retyped_operator_keeps_its_side`.

C3 SILENT. Duplicate or paste beside the last operand. `event/blocks.rs:301-304,834,
998`. `And[A, B]` seats a third filter that is invisible and dropped on save.
Refuse at capacity; gap on too many operands.
Status: FIXED `a_full_combinator_refuses_a_third_condition`, `too_many_conditions_on_a_combinator_raise_a_gap`.

C4 LOSS. A rebuild kills a focused chip field. `ui/canvas/mod.rs:279`,
`ui/canvas/drag.rs:181-198`. Lint at 0.25 s changes fault state, `same_shape` fails,
the field despawns with no submit, and later letters hit tool keys and Del deletes.
Also a press inside a text chip that moves 4 px starts a block drag. Defer rebuilds
while a `ChipInput` holds focus; stop the press walk at a `TextField`.
Status: FIXED `a_rebuild_waits_for_an_open_chip_field`, `a_press_inside_a_text_field_does_not_start_a_block_drag`.

C5. Lint on the wrong document after an asset switch. `problems.rs:418`,
`history.rs:425`, `ui/browser.rs:1474`. `switched_now` keeps the revision, so
findings stay and name dead entities. Mark Problems stale on a switch; `on_target_row`
ignores a dead target.
Status: FIXED `a_switched_document_drops_the_findings_of_the_last`, `a_row_on_a_dead_node_selects_nothing`.

C6. Ctrl shortcuts type letters into a focused field.
`crates/nova_ui/src/widget/text_field.rs:362`. No modifier check; winit behaviour
is from memory, so reproduce. Skip `Key::Character` while Ctrl or Super is held.
Status: FIXED `a_ctrl_chord_types_nothing`, `altgr_still_types`.

C7. A reporter moved left to right in one operator does nothing but makes a step.
`event/blocks.rs:447,497,510`. Resolve the target socket entity before
`refill_socket`, then fill by entity.
Status: FIXED `a_reporter_crosses_its_own_operator_both_ways`.

C8. Del on a reporter leaves a dead entity selected and can remove a comparison.
`ui/canvas/mod.rs:524-541`. Clear the selection; limit to non-comparison reporters.
Status: FIXED `del_on_a_reporter_clears_the_selection`, `del_leaves_a_comparison_standing`.

C9. Esc does not cancel a canvas drag. `lib.rs:930-951`, `ui/canvas/drag.rs:639`. Add
a first Esc rung that clears `held` and `candidate` and despawns the ghost.
Status: FIXED `a_cancelled_drag_puts_the_block_back`.

C10 LOW. A refused multi-paste lands part of the set. `clipboard.rs:89-113`. Resolve
every slot first, or undo the landed copies.
Status: FIXED `a_refused_block_paste_lands_nothing`.

No defect was found in `gizmo.rs`, `ui/hierarchy.rs`, `selection.rs`, the stage drag
and socket code, `popup.rs`, `select.rs`, `tabs.rs`, `tree.rs`, or the `position`
field. The `category` column had a diff skim only.
