//! loop_vfx_range: every combat effect the game draws, fired in one fixed
//! cycle against a target that survives it.
//!
//! The bench for `tasks/20260822-204201` (make particle effects credible in
//! vacuum). Four effects live in three files and are otherwise only ever seen
//! inside a real fight, where no two runs show the same thing: the turret
//! muzzle flash, the round impact, the torpedo launch and the detonation. This
//! example fires all four from their PRODUCTION paths - a real turret with a
//! real trigger, real rounds under the real travel rules, a real bay under a
//! scripted order - on a script, in the same order, every run.
//!
//! Two jobs, one scene, and the second is why the cycle is fixed:
//!
//! 1. **The eye.** One run shows every effect, so a tuning pass costs one
//!    build instead of one build per family.
//! 2. **The clock.** The example claims the frame-time capability, so the same
//!    cycle is the before/after number for the work that adds dynamic light.
//!    A scripted cycle replays the same events every run, which a live fight
//!    cannot do - and a VFX budget argued from a live fight is argued from
//!    noise.
//!
//! ## The range
//!
//! A shooter at the origin and a target [`RANGE`] units down -Z, both
//! controller-less so nothing flies and nothing decides anything. Every
//! section on both ships is authored to [`TOUGH_SECTION_HEALTH`], which is
//! what lets the cycle repeat: the warhead lands its full 750 on the target
//! and wounds it, and the next cycle has something left to shoot at. The
//! shooter sits outside the 30-unit blast radius by construction - see
//! [`RANGE`] - so it is never damaged by its own ordnance.
//!
//! Both ships are unmanaged (no `WeaponsHot`), which is what lets the script
//! fire the turret by writing [`TurretSectionInput`] directly instead of
//! standing up a controller and a safety.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/vfx-range.webm`.
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_vfx_range --features debug
//! ```
//!
//! Measure (the number the VFX work moves against):
//! ```text
//! cargo run --features debug -- probe run loop_vfx_range --release
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_vfx_range")]
#[command(version = "1.0.0")]
#[command(about = "Fire every combat effect once, on a fixed cycle")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "vfx-range";

const SHOOTER_ID: &str = "vfx_shooter";
const TARGET_ID: &str = "vfx_target";

/// How far the target sits down -Z from the shooter.
///
/// Above the shipped bay's 30-unit `blast_radius`, so the detonation at the
/// target end cannot reach back and chew the gun platform over repeated
/// cycles. Below the PDC's 200-unit reach by a wide margin, so a round crosses
/// in about a third of a second and the burst reads as a burst rather than as
/// a wait.
const RANGE: f32 = 36.0;

/// Health every section on both ships is authored to.
///
/// The cycle has to survive itself. One pass lands a 750 warhead plus roughly
/// a hundred PDC rounds on the target, and the range runs that pass three
/// times, so anything near prototype health dies in the first cycle and the
/// remaining two measure an empty sky.
const TOUGH_SECTION_HEALTH: f32 = 20_000.0;

/// The bay's content id. Not in `catalog_ids` - the shipped bay is authored
/// under its bare kind name.
const TORPEDO_BAY_SECTION_ID: &str = "torpedo_section";

/// Where a PDC's centre sits when it is bolted to the top of a unit hull cell.
///
/// A turret is not a cell: it is a 0.5 mount whose ONE link point is its base
/// plate, so it mates half its own size above the face rather than a full cell
/// away. Placing it at a whole unit leaves the plate a quarter unit off the
/// hull and the content lint refuses the ship outright - the graph comes back
/// disconnected with the turret alone in its own component.
const TURRET_MOUNT_Y: f32 = 0.75;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        // The frame-time claim is UNCONDITIONAL, unlike `wfc_ships`, which
        // declares one only under a capture. A posed row holds no load worth
        // grading; this range holds nothing else. Every pass of the cycle
        // fires the same four effects in the same order, so the number is
        // repeatable enough to grade, and grading it is what the example is
        // for - `probe run` is the before/after gauge the VFX work is judged
        // on. Measurement opens once both ships are standing, which is the
        // only state in which the load is the load being measured.
        app.add_plugins(
            nova_probe::nova_frametime()
                .window(90, 600)
                .ready_when(range_is_standing),
        );
        app.add_plugins(vfx_script());
        app.add_systems(Startup, hide_dev_overlays);
        // The shot resolution stands down for a MEASURED run: the frame-time
        // capture sizes the window before winit creates it, and a second
        // Startup writer asking for a capture size is both ambiguous with it
        // and refused by the window manager afterwards (`wfc_ships` records
        // the same trap). A hand-run has no such writer and keeps the frame.
        if !nova_probe::probe_armed() {
            app.add_systems(Startup, force_capture_resolution);
        }
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    let objects = vec![shooter(&sections), target(&sections)];
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Every combat effect, fired once per cycle at a target that survives it."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                objects
                    .into_iter()
                    .map(EventActionConfig::SpawnScenarioObject)
                    .collect::<Vec<_>>(),
                ThreePointRig::around("vfx range", Vec3::new(0.0, 0.0, -RANGE * 0.5), 3.0)
                    .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "vfx_range".to_string(),
            "VFX Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The gun platform: one controller, one PDC and one bay, on a spine.
///
/// The turret sits high and the bay low so the muzzle flash and the launch are
/// separable in one frame - the whole point of a range is that two effects
/// firing a second apart do not overlap on screen.
fn shooter(sections: &GameSections) -> ScenarioObjectConfig {
    let specs = [
        SectionSpec::new("spine", LIGHT_HULL_SECTION_ID, Vec3::ZERO),
        SectionSpec::new(
            "controller",
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 0.0, 1.0),
        ),
        SectionSpec::new(
            "turret",
            PDC_KINETIC_TURRET_SECTION_ID,
            Vec3::new(0.0, TURRET_MOUNT_Y, 0.0),
        ),
        SectionSpec::new("bay", TORPEDO_BAY_SECTION_ID, Vec3::new(0.0, -1.0, 0.0)),
    ];
    toughened(
        SHOOTER_ID,
        "Shooter",
        Vec3::ZERO,
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// The backstop: a slab wide enough that a spread burst lands ON it.
///
/// Three by three rather than a single section, because a PDC burst carries a
/// cosmetic spread and a one-cell target turns half the rounds into misses
/// that fly off camera - which reads as a broken gun rather than as a range.
fn target(sections: &GameSections) -> ScenarioObjectConfig {
    let mut specs: Vec<SectionSpec> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
        .map(|(x, y)| {
            SectionSpec::new(
                format!("slab_{x}_{y}"),
                LIGHT_HULL_SECTION_ID,
                Vec3::new(x as f32, y as f32, 0.0),
            )
        })
        .collect();
    specs.push(SectionSpec::new(
        "controller",
        BASIC_CONTROLLER_SECTION_ID,
        Vec3::new(0.0, 0.0, 1.0),
    ));
    toughened(
        TARGET_ID,
        "Target",
        Vec3::new(0.0, 0.0, -RANGE),
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// One ship, every section authored to [`TOUGH_SECTION_HEALTH`] and structural
/// collapse switched off.
///
/// Both knobs serve the same end and neither is enough alone: health keeps a
/// section alive under the warhead, and `collapse_threshold` keeps the ROOT
/// from folding when the graph notices how much of it is wounded.
fn toughened(
    id: &str,
    name: &str,
    position: Vec3,
    mut ship: SpaceshipConfig,
) -> ScenarioObjectConfig {
    if let ShipSource::Inline(hull) = &mut ship.hull {
        hull.collapse_threshold = Some(0.0);
        for section in &mut hull.sections {
            section.modifications = vec![SectionModification::SetHealth(TOUGH_SECTION_HEALTH)];
        }
    }
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }
}

/// The cycle, three times over.
///
/// Three and not one: the frame-time window is 600 frames, and one pass is
/// shorter than that, so a single cycle would measure a quiet sky for most of
/// the capture. Three is also what makes a leak visible to the eye - an effect
/// that fails to free its instance shows up as a range that gets slower every
/// pass.
#[cfg(feature = "debug")]
fn vfx_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut plugin = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("stand the range up")
        .enter(GameStates::Loading)
        .until(range_standing())
        .deadline(30.0)
        .add()
        .step("frame the range")
        .on_enter(frame_range)
        .until(elapsed(0.8))
        .add()
        .step("open the vfx loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .until(frames(1))
        .add();

    for pass in 1..=3 {
        plugin = plugin
            .step(format!("pass {pass}: lay the guns on"))
            .on_enter(aim_at_target)
            .until(frames(12))
            .add()
            .step(format!("pass {pass}: gun burst"))
            .on_enter(|world| set_triggers(world, true))
            .until(frames(45))
            .add()
            .step(format!("pass {pass}: cease fire, watch the rounds land"))
            .on_enter(|world| set_triggers(world, false))
            .until(frames(30))
            .add()
            .step(format!("pass {pass}: torpedo away"))
            .on_enter(order_torpedo)
            .until(frames(150))
            .add()
            .step(format!("pass {pass}: hold the aftermath"))
            .until(frames(30))
            .add();
    }

    plugin
        .step("close the vfx loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}

/// Both ships are in the world.
///
/// A plain fn and not just a predicate, because it is asked twice in two
/// shapes: the walk wants an `Arc<Predicate>` and the frame-time capture wants
/// a bare closure, and "the range is up" must not be able to mean two things.
#[cfg(feature = "debug")]
fn range_is_standing(world: &World) -> bool {
    world
        .try_query_filtered::<Entity, With<SpaceshipRootMarker>>()
        .is_some_and(|mut query| query.iter(world).take(2).count() == 2)
}

/// [`range_is_standing`] as a walk predicate.
#[cfg(feature = "debug")]
fn range_standing() -> std::sync::Arc<nova_debug::harness::Predicate> {
    std::sync::Arc::new(range_is_standing)
}

/// Side on, level with the flight path, far enough out that the whole range is
/// in frame and close enough that a muzzle flash is more than one pixel.
#[cfg(feature = "debug")]
fn frame_range(world: &mut World) {
    hide_hud(world);
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the vfx range has a camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: Vec3::new(RANGE * 0.55, RANGE * 0.22, -RANGE * 0.5),
        look_at: Vec3::new(0.0, 0.0, -RANGE * 0.5),
    });
}

/// The world-space point the target slab occupies.
const fn target_point() -> Vec3 {
    Vec3::new(0.0, 0.0, -RANGE)
}

/// Point every turret at the slab.
///
/// A commanded POINT rather than a target entity: the range wants the barrel
/// where the script says it is, not wherever a lead solve decides a moving
/// body will be. Nothing on this range moves.
#[cfg(feature = "debug")]
fn aim_at_target(world: &mut World) {
    let mut query = world.query::<&mut TurretSectionTargetInput>();
    for mut aim in query.iter_mut(world) {
        **aim = Some(target_point());
    }
}

/// Hold or release every trigger on the range.
#[cfg(feature = "debug")]
fn set_triggers(world: &mut World, firing: bool) {
    let mut query = world.query::<&mut TurretSectionInput>();
    for mut trigger in query.iter_mut(world) {
        **trigger = firing;
    }
}

/// Order one torpedo out of every bay, committed to the target ship.
///
/// The order is one-shot: the commit system consumes it once the projectile
/// exists, so a pass that re-inserts it fires exactly one more torpedo.
#[cfg(feature = "debug")]
fn order_torpedo(world: &mut World) {
    let Some(target) = target_ship(world) else {
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionConfigHelper>>()
        .iter(world)
        .collect();
    for bay in bays {
        world
            .entity_mut(bay)
            .insert(ScriptedTorpedoOrder { target });
    }
}

/// The target ship's root, by the scenario id it was spawned under.
#[cfg(feature = "debug")]
fn target_ship(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
        .iter(world)
        .find(|(_, id)| ***id == *TARGET_ID)
        .map(|(entity, _)| entity)
}
