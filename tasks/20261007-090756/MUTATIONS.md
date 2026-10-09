# Serial mutation pass (main worker)

Named by workers and the main worker. The main worker runs them one at a
time at the end: cp the file to /tmp, mutate, run the named test, cp back,
cmp. No other proof runs while one is open.

| # | File | Mutation | Test that must fail | Source |
|---|---|---|---|---|
| 1 | nova_world_base/src/save/session.rs | drop the `check_saved_ids` block | `a_save_with_a_duplicate_id_fails_and_keeps_the_last_save` | main |
| 2 | nova_world_base/src/save/mod.rs | drop the `check_saved_ids` call in `open_world` | `a_world_with_a_duplicate_id_is_refused_on_open` | main |
| 3 | nova_world_base/src/save/mod.rs | drop `\|\| part.contains('/')` | both id tests | main |
| 4 | nova_scenario/src/objects/asteroid_carve.rs | thaw always inserts `ChunkGrace::resumed` (drop the `None` arm) | `a_resumed_rock_chunk_has_its_mesh_material_and_grace` | SW-CHUNK.md round 2 |
| 5 | nova_scenario/src/objects/asteroid_carve.rs | `validate` skips the index-range check | none: no test is approved for chunk data validation; not run | SW-ACCESS.md |
| 6 | nova_ship/src/sections/fixture.rs | drop `style` from the `try_insert` in `shed_dead_fixtures` | `a_resumed_shed_fixture_keeps_its_art_style_and_grace` | SW-SHED.md round 1 |
| 7 | nova_ship/src/sections/torpedo_section/frozen.rs | `part_health` drops the zero-health check | `a_resumed_torpedo_keeps_its_target_arming_and_cold_launch` | main |
| 8 | nova_ship/src/sections/torpedo_section/frozen.rs | thaw skips the part health write (worker ran it once: failed as expected) | `a_resumed_torpedo_keeps_its_target_arming_and_cold_launch` | SW-TORPEDO.md round 2 |
| 9 | nova_world_base/src/save/transients.rs | phase 2 of `spawn_resumed` skips the `TorpedoTargetEntity` insert | `a_moving_wreck_and_the_torpedo_tracking_it_come_back_together`, `a_torpedo_tracking_a_torpedo_resumes_on_it` | SW-SAVETESTS.md |
| 10 | nova_world_base/src/save/transients.rs | phase 2 links on the next frame (store the links in a resource, insert them one frame later) | same two tests: the restore-frame assertion | main |
| 11 | nova_world_base/src/save/transients.rs | the freeze target closure skips the `indices` lookup (a transient target gives `NoDurableId`) | `a_torpedo_tracking_a_torpedo_resumes_on_it` | SW-SAVETESTS.md |
| 12 | nova_world_base/src/save/mod.rs | `check_saved_ids` skips the `Transient` self and range check | `a_world_with_a_duplicate_id_is_refused_on_open` | SW-SAVETESTS.md |
| 13 | nova_world_base/src/save/mod.rs | `check_saved_ids` skips the bay `has_section` check | `a_world_with_a_duplicate_id_is_refused_on_open` | main |
| 14 | nova_world_base/src/save/transients.rs | `resolve_all` skips the style check | `a_load_whose_piece_wears_an_unknown_style_is_refused` (shed and piece cases) | main |
| 15 | nova_world_base/src/save/transients.rs | `resolve_all` drops the detached piece resource preflight | `a_load_whose_piece_wears_an_unknown_style_is_refused` (no-style case) | main |
| 16 | nova_world_base/src/save/transients.rs | the freeze target closure returns `Unsettled` for a despawned target instead of `Ok(None)` | `a_torpedo_tracking_a_torpedo_resumes_on_it`, `leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused` (nova_menu) | main |
| 17 | nova_ship/src/sections/frozen_piece.rs | `classify_art_node` ignores `PlaceholderArtMarker` | `a_detached_piece_resumes_as_the_art_it_wore` (panics) | main |
| 18 | nova_ship/src/sections/frozen_piece.rs | the thaw skips `mark_wreck_cracks` for a cracked placeholder | `a_detached_piece_resumes_as_the_art_it_wore` (thaw step) | main |
| 19 | nova_ship/src/sections/hull_section.rs | spawn the raw `(Mesh3d, MeshMaterial3d)` pair with no marker | `a_detached_piece_resumes_as_the_art_it_wore` (hull case panics) | main |
| 20 | nova_ship/src/sections/damage_cracks.rs | `mesh_wears_cracks` reads only `MeshMaterial3d<SectionCracksMaterial>` | `a_detached_piece_resumes_as_the_art_it_wore` (pre-grade window) | main |

See also the mutations named in SW-WRECKID.md, SW-IDRULE.md, SW-PIECE.md,
SW-TORPEDO.md and SW-SHED.md.

## Results (main worker, 2026-10-09, serial)

Each run: back up the file, apply the mutation, run the named test, restore, cmp.
Runner: /tmp/nova-mut/run.py (not kept in the tree).

| # | Result | Failing test and line |
|---|---|---|
| 1 | killed | a_save_with_a_duplicate_id_fails_and_keeps_the_last_save (tests.rs:1360) |
| 2 | killed | a_world_with_a_duplicate_id_is_refused_on_open, generation 1 Ok(()) |
| 3 | killed | both id tests (generation 3 Ok(())) |
| 4 | killed | a_resumed_rock_chunk_has_its_mesh_material_and_grace (asteroid_carve.rs:1937) |
| 5 | not run | no approved test |
| 6 | killed | a_resumed_shed_fixture_keeps_its_art_style_and_grace (frozen.rs:415) |
| 7 | killed | a_resumed_torpedo_keeps_its_target_arming_and_cold_launch (frozen.rs:605) |
| 8 | killed | same test, "thaw_torpedo must write the saved controller health" left 10.0 |
| 9 | killed | wreck and torpedo-on-torpedo tests, "the restore frame links ... no extra tick" left None |
| 10 | not run | needs a deferred-link scaffold; the restore-frame assertion of #9 is the pin |
| 11 | killed | a_torpedo_tracking_a_torpedo_resumes_on_it, Failed(... has no durable id) |
| 12 | killed | a_world_with_a_duplicate_id_is_refused_on_open, generation 8 Ok(()) |
| 13 | killed | same, generation 6 Ok(()) |
| 14 | killed | a_load_whose_piece_wears_an_unknown_style_is_refused, generation 1 None |
| 15 | killed | same, panics in spawn_resumed (transients.rs:723) |
| 16 | killed | a_torpedo_tracking_a_torpedo_resumes_on_it (run_until_idle, tests.rs:353) and the nova_menu leave test (update_until, leave.rs:85) |
| 17 | killed | a_detached_piece_resumes_as_the_art_it_wore (frozen_piece.rs:463 panic) |
| 18 | killed | same (thaw step, frozen_piece.rs:1667) |
| 19 | killed | same (hull case, frozen_piece.rs:1289) |
| 20 | killed | same, pre-grade window (frozen_piece.rs:1305). First run survived the pre-grade assertion: the test ran an `app.update()` that graded the mesh. The main worker moved the freeze to the trigger flush, before any Update system. |
| W1 | killed | a_severed_ship_mints_a_distinct_id_for_each_wreck (integrity.rs:2072) |
| W2 | killed | a_wreck_severed_again_derives_its_id_from_the_wreck, left "ship_a/wreck/right" |
| W3 | killed | a_minted_id_already_live_leaves_the_wreck_idless (integrity.rs:2286) |
| I1 | killed | a_slash_in_an_object_or_section_id_is_an_error (scenario.rs:4144) |
| I2 | killed | same (scenario.rs:4114) |
| T1 | killed | a_resumed_torpedo_keeps_its_target_arming_and_cold_launch, "arming must survive" |
| P1 | killed | a_detached_piece_resumes_as_the_art_it_wore, cargo_intake door pose |

Test-only mutations named in SW-PIECE.md round 3 and SW-SHED.md round 2 are not
production mutations and were not run.
| A1 | killed | `live_by_id` maps many matches to Missing (wait): a_resumed_shot_still_belongs_to_its_shooter, "ships: refused on the first restore frame" left None |
| A2 | killed | the section lookup waits on many matches: same test, "sections: refused on the first restore frame" left None |
