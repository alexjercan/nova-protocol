//! loop_vfx_range: every combat effect the game draws, fired in one fixed
//! cycle against a target that survives it.
//!
//! The bench for `tasks/20260822-204201` (make particle effects credible in
//! vacuum) and, since `tasks/20260902-143732`, for the railgun slug's wake.
//! Five effects live in four files and are otherwise only ever seen inside a
//! real fight, where no two runs show the same thing: the turret muzzle
//! flash, the round impact, the torpedo launch, the detonation, and the slug's
//! wake with the light that rides it. This example fires all five from their
//! PRODUCTION paths - a real turret with a real trigger, real rounds under the
//! real travel rules, a real bay under a scripted order, a real lance on a
//! real commit - on a script, in the same order, every run.
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
//! A shooter at the origin and a target [`RANGE`] down -Z, both
//! controller-less so nothing flies and nothing decides anything. Every
//! section on both ships is authored to [`TOUGH_SECTION_HEALTH`], which is
//! what lets the cycle repeat: the warhead lands its full 750 on the target
//! and wounds it, and the next cycle has something left to shoot at. The
//! shooter sits outside the 300 m blast radius by construction - see
//! [`RANGE`] - so it is never damaged by its own ordnance.
//!
//! Every ship is unmanaged (no `WeaponsHot`), which is what lets the script
//! fire the turret and the lance by writing [`TurretSectionInput`] and
//! [`RailgunSectionInput`] directly instead of standing up a controller and a
//! safety.
//!
//! ## The lance
//!
//! A spinal lance is parked on its own hull above the shooter, bore down -Z,
//! so its slug passes over the target and flies its whole lifetime into empty
//! sky with the whole wake behind it. Not a section on the shooter: a lance
//! on the shooter's axis puts its slug into the target 360 m on - a tick
//! and a half of flight, over before the wake's first frame at a software
//! renderer's frame rate. The platform carries a hard magazine of one shell
//! per pass in place of the shipped one shell and twelve-second reload, and
//! is locked where it stands: the recoil is real and there is nothing on the
//! platform to counter it.
//!
//! `NOVA_VFX_RANGE_BARE_SLUG=1` strips the wake and the light off every slug
//! as they spawn. The same cycle with a bare slug is the wake's BEFORE
//! number, on the same revision as its after.
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
//! cargo run --features debug -- probe run loop_vfx_range --release --repeat 5
//! NOVA_VFX_RANGE_BARE_SLUG=1 cargo run --features debug -- \
//!   probe run loop_vfx_range --release --repeat 5 --out target/probe-bare
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::LockedAxes;
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_vfx_range")]
#[command(version = "1.0.0")]
#[command(about = "Fire every combat effect once, on a fixed cycle")]
struct Cli;

/// The wide loop: the whole cycle, guns through detonation, from the range
/// pose that keeps both ships in frame.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "vfx-range";

/// The close loop, over the last pass's torpedo only. A launch is a small
/// event at range framing - the coast is under 40 m of a 360 m
/// shot - so the one thing the release changed about it is invisible from
/// there. Two loops rather than one long one: only one may be open at a time.
#[cfg(feature = "debug")]
const LAUNCH_LOOP: &str = "vfx-cold-launch";

const SHOOTER_ID: &str = "vfx_shooter";
const TARGET_ID: &str = "vfx_target";
const LANCE_PLATFORM_ID: &str = "vfx_lance";

/// The environment variable that flies the slug bare: no wake, no light.
const BARE_SLUG_ENV: &str = "NOVA_VFX_RANGE_BARE_SLUG";

/// How high above the shooter the lance platform is parked.
///
/// The slug has to clear the target's top row with the rake sphere behind
/// it: the slab reaches 15 m up and the rake is 10 m wide, so 40 m
/// leaves 15 m of sky between the two. Higher and the platform
/// leaves the frame.
const LANCE_PLATFORM_Y: Meters = Meters(40.0);

/// The lance's hard magazine for one run: one shell per pass, no reload.
///
/// The shipped lance holds one shell behind a twelve-second idle reload, and
/// the cycle is shorter than that at any frame rate worth measuring, so on
/// the shipped magazine which passes fire is a property of the host.
const LANCE_SHELLS: u32 = 3;

/// How far the target sits down -Z from the shooter.
///
/// Above the shipped bay's 300 m `blast_radius`, so the detonation at the
/// target end cannot reach back and chew the gun platform over repeated
/// cycles. Below the PDC's 2 km reach by a wide margin, so a round crosses
/// in about a third of a second and the burst reads as a burst rather than as
/// a wait.
const RANGE: Meters = Meters(360.0);

/// Frames held between ordering the torpedo and the aftermath step.
///
/// Sized off the measured flight, not guessed: the torpedo crosses the range
/// and detonates about 57 frames after the order, so this leaves the blast a
/// sixth of a second before the aftermath step takes over and another second
/// there. It is a budget as much as a duration - three passes of
/// `12 + 45 + 30 + this + 30`, plus the frames the recorder drains on close,
/// have to fit inside the loop recorder's 600 frame cap, and a longer coast is
/// the first thing that breaks it.
#[cfg(feature = "debug")]
const TORPEDO_FLIGHT_FRAMES: u32 = 70;

/// The pass whose torpedo the close loop takes. The last one: the wide loop
/// keeps every pass before it, so nothing is lost from either.
#[cfg(feature = "debug")]
const LAUNCH_PASS: u32 = 3;

/// Frames the close loop records of the last pass's torpedo.
///
/// Short on purpose. The subject leaves the close pose about here, and the
/// rest of `TORPEDO_FLIGHT_FRAMES` is the empty sky it left behind - which on
/// a page that loops the file forever is two thirds of a clip showing nothing.
#[cfg(feature = "debug")]
const LAUNCH_LOOP_FRAMES: u32 = 60;

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

/// Where a PDC's centre sits when it is bolted to the top of a one-cell hull
/// section. BUILD-GRID cells, not meters.
///
/// A turret is not a cell: it is a 0.5 mount whose ONE link point is its base
/// plate, so it mates half its own size above the face rather than a full cell
/// away. Placing it at a whole cell leaves the plate a quarter cell off the
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
    if std::env::var_os(BARE_SLUG_ENV).is_some() {
        app.add_observer(strip_wake);
        app.add_observer(strip_slug_light);
    }
}

/// Under [`BARE_SLUG_ENV`]: take the wake off a slug the frame it is given.
fn strip_wake(add: On<Add, RailgunWakeEmitter>, mut commands: Commands) {
    commands.entity(add.entity).despawn();
}

/// Under [`BARE_SLUG_ENV`]: take the light off a slug the frame it is given.
fn strip_slug_light(add: On<Add, RailgunSlugLight>, mut commands: Commands) {
    commands.entity(add.entity).despawn();
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    let objects = vec![
        shooter(&sections),
        target(&sections),
        lance_platform(&sections),
    ];
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
                ThreePointRig::around(
                    "vfx range",
                    Meters3(Vec3::new(0.0, 0.0, -RANGE.get() * 0.5)),
                    3.0,
                )
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
        // The bay is 1x1x2: the half-cell shift puts both of its cells on the
        // grid, roof sockets against the spine and the controller.
        SectionSpec::new("bay", TORPEDO_BAY_SECTION_ID, Vec3::new(0.0, -1.0, 0.5)),
    ];
    toughened(
        SHOOTER_ID,
        "Shooter",
        Meters3::ZERO,
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
        Meters3(Vec3::new(0.0, 0.0, -RANGE.get())),
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// The lance platform: a spinal lance and the controller behind it, parked
/// [`LANCE_PLATFORM_Y`] above the shooter with its bore down -Z.
///
/// The lance is three cells long and centred on its own origin, so at -1 it
/// fills -2 to 0 and its muzzle sits half a cell ahead of the shooter's bow.
/// The controller at +1 is on the lance's aft link. Nothing stands ahead of
/// the bore: a lance cannot traverse, so anything there is shot through.
fn lance_platform(sections: &GameSections) -> ScenarioObjectConfig {
    let specs = [
        SectionSpec::new("lance", RAILGUN_LANCE_SECTION_ID, Vec3::new(0.0, 0.0, -1.0)),
        SectionSpec::new(
            "controller",
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 0.0, 1.0),
        ),
    ];
    let mut platform = toughened(
        LANCE_PLATFORM_ID,
        "Lance Platform",
        Meters3(Vec3::new(0.0, LANCE_PLATFORM_Y.get(), 20.0)),
        fixtures::ship(sections, SpaceshipController::None, &specs),
    );
    if let ScenarioObjectKind::Spaceship(ship) = &mut platform.kind {
        if let ShipSource::Inline(hull) = &mut ship.hull {
            for section in hull
                .sections
                .iter_mut()
                .filter(|section| section.id == "lance")
            {
                section
                    .modifications
                    .push(SectionModification::SetAmmo(LANCE_SHELLS));
            }
        }
    }
    platform
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
    position: Meters3,
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
        .on_enter(|world| {
            pin_lance_platform(world);
            frame_range(world);
        })
        .until(elapsed(0.8))
        .add()
        .step("open the vfx loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add();

    for pass in 1..=3 {
        plugin = plugin
            .step(format!("pass {pass}: lay the guns on"))
            .on_enter(|world| {
                aim_at_target(world);
                set_lance_trigger(world, true);
            })
            .until(frames(12))
            .add()
            .step(format!("pass {pass}: gun burst"))
            .on_enter(|world| {
                set_lance_trigger(world, false);
                set_triggers(world, true);
            })
            .until(frames(45))
            .add()
            .step(format!("pass {pass}: cease fire, watch the rounds land"))
            .on_enter(|world| set_triggers(world, false))
            .until(frames(30))
            .add();

        // The wide loop has both its torpedoes by now, so it hands the last
        // one to the close pose. The gun half of pass 3 still rides the wide
        // loop, which is why the hand-off is here and not before the pass.
        if pass == LAUNCH_PASS {
            plugin = plugin
                .step("close the vfx loop")
                .on_enter(|world| loop_end(world, LOOP_NAME))
                .until(loop_written(LOOP_NAME))
                .deadline(60.0)
                .add()
                .step("close on the bay for the launch")
                .on_enter(frame_launch)
                .until(frames(6))
                .add()
                .step("open the cold launch loop")
                .on_enter(|world| loop_start(world, LAUNCH_LOOP))
                .add();
        }

        plugin = plugin
            .step(format!("pass {pass}: torpedo away"))
            .on_enter(order_torpedo)
            .until(frames(if pass == LAUNCH_PASS {
                LAUNCH_LOOP_FRAMES
            } else {
                TORPEDO_FLIGHT_FRAMES
            }))
            .add();

        if pass == LAUNCH_PASS {
            plugin = plugin
                .step("close the cold launch loop")
                .on_enter(|world| loop_end(world, LAUNCH_LOOP))
                .until(loop_written(LAUNCH_LOOP))
                .deadline(60.0)
                .add()
                .step(format!("pass {pass}: the rest of the flight"))
                .until(frames(TORPEDO_FLIGHT_FRAMES - LAUNCH_LOOP_FRAMES))
                .add();
        }

        plugin = plugin
            .step(format!("pass {pass}: hold the aftermath"))
            .until(frames(30))
            .add();
    }

    plugin
}

/// All three ships are in the world.
///
/// A plain fn and not just a predicate, because it is asked twice in two
/// shapes: the walk wants an `Arc<Predicate>` and the frame-time capture wants
/// a bare closure, and "the range is up" must not be able to mean two things.
#[cfg(feature = "debug")]
fn range_is_standing(world: &World) -> bool {
    world
        .try_query_filtered::<Entity, With<SpaceshipRootMarker>>()
        .is_some_and(|mut query| query.iter(world).take(3).count() == 3)
}

/// Lock the lance platform where it stands.
///
/// Its recoil is the shipped 45, on a hull of four cells and nothing to
/// counter it: unlocked, every shot sends the platform a ship's length a
/// second backwards, out of the frame and out of the cycle. Locked axes and
/// not a static body, so the ship keeps the body every other part of it was
/// built against.
#[cfg(feature = "debug")]
fn pin_lance_platform(world: &mut World) {
    if let Some(platform) = ship_by_id(world, LANCE_PLATFORM_ID) {
        world.entity_mut(platform).insert(LockedAxes::ALL_LOCKED);
    }
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
    // Offsets FROM the midpoint, not absolute: the whole frame is sized off
    // RANGE, so the shot survives the range being lengthened. The distance
    // that comes out is a little over 1 x RANGE, which at bevy's default 45
    // degree vertical fov and 16:9 leaves both ships clear of the edges with
    // room for the ejecta thrown PAST the target - the fragments reach further
    // than the hull they came off, and framing to the hulls crops them.
    // Further out than this and a detonation is a bright speck.
    let midpoint = Meters3(Vec3::new(0.0, 0.0, -RANGE.get() * 0.5));
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: midpoint
            + Meters3(Vec3::new(
                RANGE.get() * 0.98,
                RANGE.get() * 0.28,
                RANGE.get() * 0.22,
            )),
        look_at: midpoint,
    });
}

/// Close on the shooter's bow, along the way the torpedo leaves.
///
/// The bay ejects out of the ship's -Z and the coast is about 38 m, so
/// the frame is sized to that rather than to `RANGE`: the drop, the inert
/// travel and the moment the drive catches all have to happen inside it. It
/// is centred a coast and a burn down range rather than on the bay, so the lit
/// torpedo is still in shot when `LAUNCH_LOOP_FRAMES` runs out. Held off the
/// launch axis rather than on it, because a torpedo coming straight at the
/// camera does not read as travelling at all.
#[cfg(feature = "debug")]
fn frame_launch(world: &mut World) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the vfx range has a camera");
    let look_at = Meters3::new(0.0, 0.0, -70.0);
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: look_at + Meters3::new(110.0, 32.0, 65.0),
        look_at,
    });
}

/// The world-space point the target slab occupies, in ENGINE units - a
/// commanded turret aim point is engine world space.
#[cfg(feature = "debug")]
fn target_point() -> Vec3 {
    Vec3::new(0.0, 0.0, -RANGE.to_engine())
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

/// Hold or release every turret trigger on the range.
#[cfg(feature = "debug")]
fn set_triggers(world: &mut World, firing: bool) {
    let mut query = world.query::<&mut TurretSectionInput>();
    for mut trigger in query.iter_mut(world) {
        **trigger = firing;
    }
}

/// Hold or release the lance's trigger.
///
/// Held for the whole lay-on step rather than pulsed: the walk runs on the
/// render clock and the gun on the fixed one, so a one-frame tap can fall
/// between two of the gun's ticks. The commit outlives the release, and the
/// shot leaves when the charge arrives, a second and a half on - inside the
/// burst, with the rounds still landing.
#[cfg(feature = "debug")]
fn set_lance_trigger(world: &mut World, held: bool) {
    let mut query = world.query::<&mut RailgunSectionInput>();
    for mut trigger in query.iter_mut(world) {
        trigger.0 = held;
    }
}

/// Order one torpedo out of every bay, committed to the target ship.
///
/// The order is one-shot: the commit system consumes it once the projectile
/// exists, so a pass that re-inserts it fires exactly one more torpedo.
#[cfg(feature = "debug")]
fn order_torpedo(world: &mut World) {
    let Some(target) = ship_by_id(world, TARGET_ID) else {
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

/// A ship's root, by the scenario id it was spawned under.
#[cfg(feature = "debug")]
fn ship_by_id(world: &mut World, id: &str) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
        .iter(world)
        .find(|(_, entity_id)| ***entity_id == *id)
        .map(|(entity, _)| entity)
}
