# L11 - Perf and small correctness

**Baseline: NEUTRAL.** Behavior-only.

Findings: **F37, F38** (the two that matter), **F24, F25, F26, F27, F35, F36,
F43, F44, F62, F64, F65, F66, F67, F72, F82, F85, F86**.

**Depends on:** L1 - F37 in particular sits directly under the probe's FPS
baseline check, so its before/after evidence is only meaningful once the gate
is trustworthy.

## F37 - the largest single performance defect in the review

```rust
// crates/nova_gameplay/src/sections/turret_section/render.rs:126-133
None => {
    commands.entity(entity).insert((children![(
        Name::new("Bullet Projectile Render"),
        Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.3))),        // <- new Mesh
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.9, 0.2))), // <- new Material
    ),],));
}
```

**This is the production path, not a fallback.** Every shipped turret sets
`projectile_render_mesh: None` (`nova_assets/src/sections.rs:286,360,390`,
`scenario/craft.rs:257,275,345`). The default turret fires 100 rounds/s per
muzzle, so one held trigger creates **100 mesh assets and 100 material assets
per second**, each forcing a GPU buffer upload and a fresh bind group /
pipeline specialization.

```rust
// NEW  crates/nova_gameplay/src/sections/turret_section/render.rs
/// The default bullet's mesh and material, created once. The None arm of
/// insert_projectile_render is the SHIPPED path (every stock turret authors
/// no projectile mesh), so allocating per bullet allocated per shot.
#[derive(Resource)]
pub struct DefaultProjectileRender {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
//   init in the plugin's startup; the None arm clones two handles.
```

## F38 - best cost/benefit ratio in the review

```rust
// crates/nova_gameplay/src/flight/autopilot.rs:877
// crates/nova_gameplay/src/flight/manual.rs:142
//   BYTE-IDENTICAL for 16 lines:
for (thruster, mut input, _, _, &ChildOf(parent)) in &mut q_thruster {
    if parent != ship { continue; }
    let target = allocation
        .iter()
        .position(|(e, _)| *e == thruster)     // <- O(allocation) inside the loop
        .map(|i| throttles[i])
        .unwrap_or(0.0);
    **input = spool(**input, target, settings.spool_up_rate, settings.spool_down_rate, dt);
}
//   For every ship, walk every unbound thruster IN THE WORLD and run
//   position() inside that loop:
//   O(ships x thrusters x thrusters_on_this_ship), EVERY FixedUpdate TICK.
//   This is the workspace's only real per-tick complexity bug, and there are
//   two copies of it.
```

```rust
// NEW  crates/nova_gameplay/src/flight/spool.rs  (or wherever `spool` lives)
/// Drive every thruster of `ship` toward its allocated throttle, zero for the
/// ones the allocation left dark. Extracted from autopilot.rs:877 and
/// manual.rs:142, which were byte-identical and both O(n^2) per tick.
pub(crate) fn spool_allocated_thrusters(
    ship: Entity,
    allocation: &[(Entity, BalanceEngine)],
    throttles: &[f32],
    q_thruster: &mut Query<...>,
    settings: &FlightSettings,
    dt: f32,
);
//   Build a HashMap<Entity, usize> from `allocation` once, outside the loop.
```

**One extraction kills the duplicate and both copies of the bug together. Do
not fix the complexity bug in place in two files.**

Safety note, already verified: `balance_throttles` always returns
`engines.len()` entries, so the `throttles[i]` indexing at `autopilot.rs:884`
and `manual.rs:149` **cannot panic**. The extraction must preserve that
invariant, not add a bounds check that hides its loss.

## F24 - AI DPS varies with framerate

```rust
// crates/nova_gameplay/src/input/ai/mod.rs:107
//   The whole AI chain is registered in Update while guns.rs:119,
//   behavior.rs:292-308 and torpedo.rs:158 tick firing-gate Timers off
//   time.delta_secs() - and the firing itself happens in FixedUpdate.
//   The only sweep finding with a player-visible GAMEPLAY effect.
//   FIX: move the firing-gate timers to FixedUpdate, or tick them off
//   Time<Fixed>. Decide which by reading what else the AI chain needs from
//   Update (input sampling, probably nothing).
```

Context, so nobody widens this: the 6-vs-119 `FixedUpdate`/`Update` ratio is
**not** a problem. Everything touching avian is already fixed-stepped.

## Cluster - `nova_menu/src/settings.rs`

F26 and F27 here, F22 in L3 (data loss). **Three defects, one file.** Whoever
opens it carries all three, whichever lane the commits land in.

```rust
// crates/nova_menu/src/settings.rs:95  (same at pause.rs:203,286)   (F26)
//   Raw Text spans with no nova_ui::widget::UiText marker, so apply_ui_font
//   never routes them through UiFont. The "Volume" label, the NN% readout, the
//   Controls headers and both keybind columns render in BEVY'S DEFAULT FACE
//   beside siblings in Iosevka Term. settings.rs and pause.rs are the only
//   menu files that never import UiText. Visible in any screenshot.

// crates/nova_menu/src/settings.rs:228                              (F27)
//   The load path clamps master_volume but writes nova_os_bright_detent /
//   nova_os_scan_detent straight through. components.rs:156 clamps on read so
//   the screen looks right, but advance (:178) computes (99+1) % 4 == 0, so
//   the next BRIGHT click jumps from brightest to dimmest.
//   FIX: clamp on load, like the volume beside it.
```

## Cluster - where the phosphor and hardware skins diverge

**One investigation, five sites, one skin-comparison screenshot test.** F50 is
a deletion and sits in L5; the *reading* should happen once.

| Site | Divergence |
| --- | --- |
| `nova_ui/src/widget/button.rs:496` (F25) | `button_on_setting` fires on `On<Add, Pressed>` (mouse-DOWN) while every other button commits on `Activate` (release-over). Press a UI-skin option, drag off, release: the skin already changed, no cancel |
| `nova_ui/src/widget/panel.rs:112` (F50, **L5**) | `panel_head` discards its `skin` |
| `button.rs:244` | paint nit |
| `slider.rs:26` | paint nit |
| `slider.rs:78` | paint nit |

## Cluster - `torpedo_section/projectile.rs`, 90 lines, three findings

F23 is in L4. Read the file once.

```rust
// projectile.rs:94                                                  (F65)
//   Two unordered systems in the same schedule both plain-despawn() the same
//   torpedo (torpedo_detonate_system in SpaceshipSectionSystems,
//   update_temp_entities in TempEntitySystems::Sync, no edge, no flush). A
//   torpedo whose lifetime expires on the frame it fuzes gets two queued
//   despawns: the second warns, or HARD-PANICS under the
//   FallbackErrorHandler(panic) the autopilot and probe runs install.
//   FIX: try_despawn - the sibling despawn_shot_down_torpedoes (:43) already
//   uses it. An ordering edge between the two sets is the better fix.

// projectile.rs:65                                                  (F66)
//   torpedo_detonate_system requires &TorpedoTargetPosition, which a
//   dumb-fired torpedo never receives (intent.rs:209 inserts
//   TorpedoTargetChosen alone when CombatLock is None). A NO-LOCK LAUNCH IS
//   PHYSICALLY INCAPABLE OF DETONATING - it flies the full 100 s lifetime,
//   deals a contact ding, and is silently deleted. The bay still spent the
//   round.
//   RULED 2026-08-07: INTENDED. A no-lock launch is a MISFIRE and stays one.
//   Do not change the behavior. The only work is one comment at :65 saying so,
//   because "nothing says so" is what made this a finding - without it the
//   next reviewer re-reports it.
```

## The rest

| Finding | Site | Change |
| --- | --- | --- |
| F35 | `nova_scenario/src/objects/area.rs:53` | `forget_area_occupancy` prunes only when the AREA despawns, so a body destroyed *inside* a live area pins the count forever and **a scenario gating on `OnExit` never advances**. Also clear `AreaOccupancy` in `teardown_scenario_entities` |
| F36 | `nova_scenario/src/lint/scenario.rs:291,348` | `(0.0..=MAX).contains(&secs)` admits `0.0` while the message claims `(0, MAX]`. `auto_advance_secs: Some(0.0)` lints clean, then `outcome.rs:217` builds a Timer that finishes on its first tick |
| F43 | `nova_gameplay/src/hud/readout.rs:207` | two Strings allocated per readout per frame **before** the `if existing.0 != text` compare that throws them away. Move the allocation to the right side of the compare |
| F44 | 14 sites (`flight_status.rs:204`, `torpedo_target.rs:180`, `turret_lead.rs:222`, `damage_tint.rs:473,638`, `nova_os_map/scene.rs:104`, `nova_os_ship/scene.rs:213`, ...) | `redundant_clone` in per-frame HUD systems. Mechanical |
| F62 | `nova_gameplay/src/camera/skybox.rs:118` | `images.get_mut(&config.cubemap).unwrap()` inside an `On<Insert, SkyboxConfig>` observer, one line after a `let Ok(..) else { error!; return }` for the query. Same treatment |
| F64 | `nova_info/build.rs:11-13` | `expect("failed to get git revision")` + `unwrap()` **breaks the build** in a tarball export with no git. Fall back to `"unknown"` |
| F67 | `nova_gameplay/src/sections/thruster_section.rs:353` | main-drive thrust is a raw impulse never multiplied by `dt`, so linear authority is proportional to tick rate while torque and RCS are not. **Halving `Time<Fixed>` halves every ship's linear acceleration.** Internally consistent today - fix it or document it, but do not leave it undocumented |
| F72 | `nova_scenario/src/loader/mod.rs:144` | `ScenarioConfig::default()` is invalid by its own doc (`:141`). Add `ScenarioConfig::new(id, name, cubemap)` and delete the `Default` impl - 15 sites, mechanical |
| F82 | 5 `needless_pass_by_ref_mut` (`chip_layout_rig.rs:278`, `ai/behavior.rs:909`, `component_lock.rs:403`, `radar.rs:387`, `turret_section/aim.rs:510`) | In Bevy a `&mut` never used mutably, **if it reaches a system signature**, declares a write the scheduler serializes against. Verified: `chip_layout_rig.rs:278` is a test helper, not a system param - **read the other four before acting** |
| F85 | `while_float` x2 (`nova_os_map/tests.rs:842`, `nova_os_ship/tests.rs:1316`); `iter_with_drain` (`mesh/explode.rs:200`); `case_sensitive_file_extension_comparisons` (`run_report/artifacts.rs:81`) | float loop conditions can spin forever on a NaN; the extension comparison is irrelevant on Linux CI, real on a case-insensitive filesystem |
| F86 | `transform/directional_sphere_orbit.rs:121` (angle lerp with no wrap handling, latent only because the velocity HUD passes `smoothing: 0.0`); `math.rs:35` (absolute `f32::EPSILON` snap threshold that can never be true at chase-camera scale); `camera/shake.rs:295-296` (offset and kick fed the **same** random sample, so shake reads as 1-D jitter) | **Or drop.** None is player-visible today |

## Verified by

`probe run` with `--baseline` for F37 and F38 - **both should show a measurable
FPS improvement, and that measurement is the point.** The rest are
unit-testable.
