# Scout: transient-gate stop triggers

Source: bevy-systems scout `scout-triggers`, read-only. Saved by the main
worker. Main-worker notes are marked "Note".

## 1. Stamp order at detach: HOLDS

- `detach_destroyed_body` is in `crates/nova_gameplay/src/integrity/explode.rs:343-478`.
  It queues in this order:
  1. the piece spawn with `DetachedPieceMarker(entity)` (420-448);
  2. the reparent (461-465) and the collider strip (466-471);
  3. the source despawn (477).
- An `On<Add, DetachedPieceMarker>` observer runs while the spawn command
  applies, before commands 2 and 3. At that time the source, its
  `SectionBuildConfig` and its `ChildOf` chain still exist.
- `SectionBuildConfig` (`base_section.rs:545`) is never removed before
  despawn. `integrity.rs:226` already relies on it.
- `ShipStyle` (`skin_style.rs:577`) is found by a `ChildOf` walk, as
  `worn_style` does (`shell_skin.rs:1113-1124`).
- Whole-ship death does not use `detach_destroyed_body`:
  - The root path is `explode.rs:217-236` and `core.rs:413-424`, which
    `Without<IntegrityRoot>` at `explode.rs:356` keeps out.
  - If the root is despawned first, the section insert does nothing, so no
    piece spawns.
- The cascade peels one section per tick (`integrity.rs:1140`).
- Gap: `SectionConfig.collider` is `Option<SectionCollider>`, and `None`
  means the unit cube (`base_section.rs:365-374`). The stamp maps `None` to
  that default.

## 2. Chunk mesh availability: HOLDS

- Chunk meshes come from `TriangleMeshBuilder::build`
  (`nova_gameplay/src/mesh/builder.rs:304-326`), with
  `RenderAssetUsages::default()` (MAIN_WORLD | RENDER_WORLD). An Update
  system can read the positions, normals and indices.
- `sever_piece` (`asteroid_carve.rs:495-521`) keeps no field.
- The asteroid material reads its texture through the extension, not the
  mesh UVs (`asteroid.rs:787-858`, comment 815-819). It is rebuilt from the
  texture, kind and seed.
- Save size is unmeasured; P-T7 measures it.

## 3. Effect emit-on-start: HOLDS for piece children

| Effect | Setting | On a piece |
|---|---|---|
| Turret muzzle (`turret_section/render.rs:547-550`) | once, `emit_on_start(false)` | yes |
| Torpedo launch (`torpedo_section/render.rs:668-671`) | once, `emit_on_start(false)`, reset at `795-850` | yes |
| Torpedo detonation (`torpedo_section/render.rs:260-266`, `389`) | `emit_on_start(true)` | no, standalone entity (D-T6) |
| Pyre (`pyre.rs:495`, `571`) | `emit_on_start(true)` | no, on the dying hull (D-T6) |
| Railgun wake (`wake.rs:635`, `714`) | `rate(0.0)` ramped by properties | no, own entity |
| Mining sparks (`mining.rs:672`) | `emit_on_start(false)` | not found under a piece |
| Carve chips (`spew.rs:491`) | `emit_on_start(false)` | no (D-T6) |

No thruster `SpawnerSettings` were found. The plume claim in gate 4.6
(`thruster_section.rs:641-696`) stays unverified there.

Note: the scout's "PARTIAL" covers only effects that are not piece children.
For the gate trigger (an emitter under a piece that emits on start), the
verdict is HOLDS.

## 4. TempEntity re-insert callers: HOLDS

- `on_insert_temp_entity` (`lifetime.rs:92-114`) is `On<Insert, TempEntity>`.
- Every `TempEntity(` site in `crates/` and `examples/` inserts on a fresh
  spawn, except `railgun_section/wake.rs:435`.
- `wake.rs:435` inserts on an emitter that was spawned without `TempEntity`
  (`wake.rs:272-280`; test at `822-856`). The `retiring` flag lets it run
  once (`415-422`).
- No caller relies on the reset.
