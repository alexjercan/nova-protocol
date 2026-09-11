//! system_hud_shell: the directional-HUD shells around the live hull, verified
//! on the smallest and the largest ship in the fleet.
//!
//! Task 20260909-212917. The velocity sphere and the gravity sphere used to be
//! authored at 50 m and 56 m, which is a promise only a small hull keeps: the
//! industrial carrier is 360 m stem to stern and wore both shells buried inside
//! itself. They are now derived from the hull's own `HullEnvelopeRadius` about
//! its live centre of mass, and the flight chips park off the outer one.
//!
//! The range runs the same round twice - `block_carrier` as the player hull,
//! then `block_skiff` - because the whole defect was a number that worked at
//! one size. The big hull goes FIRST so the run's one appended picture is of
//! the small one: under the software rasterizer CI renders with, thirty settle
//! frames of 2 081 sections cost more than every assertion in this file put
//! together. Flat space on purpose: the gravity shell is HIDDEN with no well to
//! point at, and the chips still have to park off its radius, which is the half
//! of the contract a lit shell would hide.
//!
//! | # | invariant | what it pins |
//! |---|---|---|
//! | 1 | `outcome: both shells enclose the live hull` | each sphere is centred on the live COM and stands its authored clearance outside the published envelope |
//! | 2 | `outcome: the flight chips clear the outer shell` | both chips' near edge is 12 px past the projected gravity shell, with their authored rows unchanged |
//! | 3 | `outcome: severing the hull shrinks its shells` | the envelope and both radii follow the hull down |
//! | 4 | `outcome: the shells stay nested through the shrink` | mid-convergence the gravity shell is still outside the velocity shell, and the velocity shell still outside the hull |
//!
//! Controls: none needed; the run drives itself.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_hud_shell --features debug
//! # beats: load the carrier, settle its shells, assert 1-2, sever the sections
//! # that decide its envelope, assert 3-4; reload as the skiff and do it
//! # again. A beat that never resolves inside its deadline is an error exit
//! # naming the BEAT.
//! ```

#[path = "../screenshots/shared/kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_hud_shell")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the directional-HUD shells around the live hull. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The player ship's scenario id, the same in both rounds so the reload
/// replaces the hull rather than adding one.
const SHIP_ID: &str = "hud_shell_ship";

/// The small hull: the first round's player ship.
#[cfg(feature = "debug")]
const SKIFF: &str = "block_skiff";

/// The big hull: the first round's player ship, and the one the fixed 56 m
/// shell was buried inside.
const CARRIER: &str = "block_carrier";

/// How far a derived radius may sit from the contract it is checked against.
/// The shells are eased every frame, so a reading taken one frame after the
/// envelope moved is legitimately a hair off; 10 cm is far under the 5 m gap
/// the assertion is about.
#[cfg(feature = "debug")]
const RADIUS_TOLERANCE: Meters = Meters(0.1);

/// How far a sphere's centre may sit from the live centre of mass. The widget
/// pose is written in Update off the interpolated hull pose and read back a
/// frame later, so a hull under way is legitimately a frame behind; a metre
/// covers that and still fails a shell centred on the root origin, which on a
/// block hull is cells away from the mass.
#[cfg(feature = "debug")]
const CENTRE_TOLERANCE: Meters = Meters(1.0);

/// How far a chip's near edge may sit inside its authored gap. `Content` chips
/// are placed off LAST frame's measured width, so a speed readout whose digits
/// just changed is off by half that change for one frame; 8 px covers a digit
/// and is still well under the 12 px gap.
#[cfg(feature = "debug")]
const CHIP_TOLERANCE_PX: f32 = 8.0;

/// The authored gap between the projected outer shell and a chip's near edge.
#[cfg(feature = "debug")]
const CHIP_GAP_PX: f32 = 12.0;

/// The speed chip's authored row, px above the centre of mass (screen y grows
/// downward).
#[cfg(feature = "debug")]
const SPEED_ROW_PX: f32 = -90.0;

/// The mode chip's authored row.
#[cfg(feature = "debug")]
const MODE_ROW_PX: f32 = -114.0;

/// How close to the furthest live section a section has to be to count as one
/// that DECIDES the envelope.
///
/// Half a build-grid cell, so the cut takes the whole tip rather than the one
/// plate that happened to reach furthest: a hull with a symmetric bow and stern
/// has two such ends and severing one of them only moves the centre of mass,
/// and a rounded bow has a cluster of near-ties behind its furthest corner that
/// would leave the envelope all but where it was.
#[cfg(feature = "debug")]
const ENVELOPE_TIE: Meters = Meters(5.0);

/// How close the eased shell envelope has to be to the published one before a
/// round calls it converged. The sizing pass snaps inside a centimetre, so this
/// only has to be wider than that.
#[cfg(feature = "debug")]
const CONVERGED: Meters = Meters(0.02);

/// Where the range's autopilot is told to go. Far enough that the leg never
/// arrives: the mode chip shows only while the computer is flying, and a STOP
/// on a ship already at rest disengages on the tick it is given.
#[cfg(feature = "debug")]
const GOTO_TARGET: Meters3 = Meters3::new(0.0, 0.0, -50_000.0);

/// The envelope the skiff round waits to be UNDER before it believes the
/// reload happened. The skiff is a handful of cells and the carrier is 360 m
/// stem to stern, so no carrier reading can fall this far - not even the one
/// taken after the first round has cut its bow off.
#[cfg(feature = "debug")]
const SKIFF_ENVELOPE_CEILING: Meters = Meters(80.0);

/// In-step seconds a beat of the CARRIER round gets to reach its world
/// condition.
///
/// Over the fleet's [`STEP_DEADLINE_SECS`] because the hull is the reason: CI
/// runs this range on a software rasterizer, where one frame of 2 081 sections
/// costs seconds, and the beats here wait on a solve that needs several of
/// them. Measured at 12 s per convergence beat on four cores under lavapipe.
/// A backstop that names a hung beat, not a budget the round is held to.
#[cfg(feature = "debug")]
const SHELL_STEP_DEADLINE_SECS: f32 = 90.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// What the round recorded before it cut the hull, so the shrink beat compares
/// against a measured number instead of a guessed one.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ShellProbe {
    /// `(envelope, velocity radius, gravity radius)` at the moment of the cut.
    before: Option<(f32, f32, f32)>,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<ShellProbe>();
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants + frame-time capture, so
        // `probe run` grades this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(nova_screenshot(shell_script()));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(shell_range(&game_assets, &ships, CARRIER)));
}

/// The range scenario: one player hull, parked in flat space, under the photo
/// rig so the appended screenshot beat has something lit to shoot.
fn shell_range(game_assets: &GameAssets, ships: &GameShips, hull: &str) -> ScenarioConfig {
    let ship = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SHIP_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: BTreeMap::new(),
                speed_cap: None,
            }),
            allegiance: None,
            hull: ShipSource::Inline(kit::catalog_ship(ships, hull)),
            ..default()
        }),
    });

    ScenarioConfig {
        description: "One player hull for the HUD shell range.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![ship],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "hud_shell_range".to_string(),
            "HUD Shell Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: one round on the carrier, a reload, the same round on the
/// skiff.
///
/// Not the stock `nova_autopilot()` preset: every beat waits on the value it
/// depends on - the published envelope, the sized shells, the envelope coming
/// down after the cut - so a slow load or a slow solve delays the walk instead
/// of truncating it. The per-step deadlines NAME the beat that stalled.
#[cfg(feature = "debug")]
fn shell_script() -> Script {
    let script = Script::new()
        .step("load the carrier")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(SHELL_STEP_DEADLINE_SECS)
        .add();
    let script = shell_round(script, CARRIER);
    let script = script
        // A rig that is DEMONSTRABLY the fresh one: only the skiff publishes an
        // envelope this small. Not "wait for the ship to be gone, then back" -
        // the teardown and the respawn land in one flush, so there is no frame
        // in between to observe.
        .step("reload as the skiff")
        .on_enter(reload_as_skiff)
        .until(envelope_at_most(SKIFF_ENVELOPE_CEILING))
        .deadline(SHELL_STEP_DEADLINE_SECS)
        .add();
    shell_round(script, SKIFF)
}

/// One full pass of the four invariants on whichever hull is flying, appended
/// to `script`. Called once per hull, which is what makes the set a claim about
/// SIZE rather than about one ship.
#[cfg(feature = "debug")]
fn shell_round(script: Script, hull: &'static str) -> Script {
    script
        // The shells are sized off the LIVE section set, and a block hull's
        // cladding lands over several frames; wait for both spheres to have a
        // radius rather than for a guessed settling time.
        .step("settle the shells")
        .until(shells_converged())
        .deadline(SHELL_STEP_DEADLINE_SECS)
        .add()
        // One extra beat for the chips: their placement reads the chip's own
        // laid-out width, so the first frame after a resize is a frame behind.
        .step("engage the autopilot for the mode chip")
        .on_enter(engage_autopilot)
        .until(elapsed(0.5))
        .add()
        .step("assert the shells enclose the hull")
        .on_enter(move |world: &mut World| assert_shells_enclose(world, hull))
        .until(elapsed(0.2))
        .add()
        .step("assert the chips clear the outer shell")
        .on_enter(move |world: &mut World| assert_chips_clear(world, hull))
        .until(elapsed(0.2))
        .add()
        .step("sever the sections that decide the envelope")
        .on_enter(sever_envelope_sections)
        .until(envelope_came_down())
        .deadline(SHELL_STEP_DEADLINE_SECS)
        .add()
        // The shells ease IN, so the reading that proves they followed the hull
        // is the one taken after they arrive.
        .step("let the shells converge")
        .until(shells_converged())
        .deadline(SHELL_STEP_DEADLINE_SECS)
        .add()
        .step("assert the shells followed the hull down")
        .on_enter(move |world: &mut World| assert_shells_shrank(world, hull))
        .until(elapsed(0.2))
        .add()
}

/// Re-trigger the scenario load with the small hull, and clear the round state
/// the fresh rig has to re-establish.
#[cfg(feature = "debug")]
fn reload_as_skiff(world: &mut World) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let ships = world.resource::<GameShips>();
        shell_range(game_assets, ships, SKIFF)
    };
    world.resource_mut::<ShellProbe>().before = None;
    info!("shell range: reloading as the skiff");
    world.trigger(LoadScenario(config));
}

/// The player ship root. Mandatory: every assertion below is about it.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
        .expect("shell range: no player ship root")
}

/// The hull's published containment radius, world units.
#[cfg(feature = "debug")]
fn envelope_of(world: &mut World, ship: Entity) -> f32 {
    **world
        .get::<HullEnvelopeRadius>(ship)
        .expect("shell range: the hull publishes no envelope")
}

/// The hull's live centre of mass in world space - the point both spheres are
/// supposed to be centred on.
#[cfg(feature = "debug")]
fn live_com(world: &mut World, ship: Entity) -> Vec3 {
    let transform = *world
        .get::<Transform>(ship)
        .expect("shell range: the hull has a Transform");
    let center_of_mass = world
        .get::<avian3d::prelude::ComputedCenterOfMass>(ship)
        .copied();
    live_structure_anchor(&transform, center_of_mass.as_ref())
}

/// One shell's radius and the world centre of the sphere it draws.
///
/// The sphere child sits at local `(0, 0, radius)` under a root parked a radius
/// out along the read direction, so transforming that point by the root's pose
/// is where the sphere actually IS.
#[cfg(feature = "debug")]
fn shell_state(world: &mut World, source: VelocityHudSource) -> (f32, Vec3, Visibility) {
    let (radius, pose, visibility) = world
        .query_filtered::<(
            &DirectionalSphereOrbit,
            &GlobalTransform,
            &Visibility,
            &VelocityHudSource,
        ), With<VelocityHudMarker>>()
        .iter(world)
        .find(|(_, _, _, widget_source)| **widget_source == source)
        .map(|(orbit, pose, visibility, _)| (orbit.radius, *pose, *visibility))
        .unwrap_or_else(|| panic!("shell range: no {source:?} shell widget"));
    (
        radius,
        pose.transform_point(Vec3::new(0.0, 0.0, radius)),
        visibility,
    )
}

/// Both shells have a radius AND that radius has caught up with the hull.
///
/// Not just "measured": a shrinking shell eases in over a 150 ms half-life, so
/// a hull whose cladding is still landing has shells legitimately wider than
/// its published envelope. Asserting the authored clearances against a shell
/// mid-ease measures the ease, not the contract.
#[cfg(feature = "debug")]
fn shells_converged() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let sized = world
            .iter_entities()
            .filter(|entity| entity.contains::<VelocityHudMarker>())
            .filter(|entity| {
                entity
                    .get::<DirectionalSphereOrbit>()
                    .is_some_and(|orbit| orbit.radius > 0.0)
            })
            .count();
        let Some(envelopes) = world.get_resource::<HudShellEnvelopes>() else {
            return false;
        };
        let hulls: Vec<(Entity, f32)> = world
            .iter_entities()
            .filter(|entity| entity.contains::<PlayerSpaceshipMarker>())
            .filter_map(|entity| {
                entity
                    .get::<HullEnvelopeRadius>()
                    .map(|envelope| (entity.id(), **envelope))
            })
            .collect();
        sized == 2
            && !hulls.is_empty()
            && hulls.iter().all(|&(hull, published)| {
                envelopes
                    .get(&hull)
                    .is_some_and(|&eased| (eased - published).abs() <= CONVERGED.to_engine())
            })
    })
}

/// The live hull has published an envelope no larger than `ceiling`.
#[cfg(feature = "debug")]
fn envelope_at_most(
    ceiling: Meters,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let ceiling = ceiling.to_engine();
    std::sync::Arc::new(move |world: &World| {
        world
            .iter_entities()
            .filter(|entity| entity.contains::<PlayerSpaceshipMarker>())
            .filter_map(|entity| entity.get::<HullEnvelopeRadius>())
            .any(|envelope| **envelope <= ceiling)
    })
}

/// The envelope has come down from the reading the cut was taken against.
#[cfg(feature = "debug")]
fn envelope_came_down() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some((before, ..)) = world
            .get_resource::<ShellProbe>()
            .and_then(|probe| probe.before)
        else {
            return false;
        };
        world
            .iter_entities()
            .filter(|entity| entity.contains::<PlayerSpaceshipMarker>())
            .filter_map(|entity| entity.get::<HullEnvelopeRadius>())
            .any(|envelope| **envelope < before - CONVERGED.to_engine())
    })
}

/// Engage the autopilot so the mode chip is on: it shows only while the
/// computer is flying, and the range has to place both chips.
#[cfg(feature = "debug")]
fn engage_autopilot(world: &mut World) {
    let ship = player_root(world);
    world
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: GOTO_TARGET.to_engine(),
        }));
}

/// Invariant 1: both spheres are centred on the live centre of mass and stand
/// their authored clearances outside the published envelope.
#[cfg(feature = "debug")]
fn assert_shells_enclose(world: &mut World, hull: &str) {
    let ship = player_root(world);
    let envelope = envelope_of(world, ship);
    let com = live_com(world, ship);
    let (velocity_radius, velocity_centre, _) = shell_state(world, VelocityHudSource::Velocity);
    let (gravity_radius, gravity_centre, gravity_visibility) =
        shell_state(world, VelocityHudSource::Gravity);

    for (what, centre) in [("velocity", velocity_centre), ("gravity", gravity_centre)] {
        let drift = Meters::from_engine(centre.distance(com));
        assert!(
            drift < CENTRE_TOLERANCE,
            "shell range ({hull}): the {what} sphere is centred {drift:?} from the live \
             centre of mass {com:?}"
        );
    }

    let hull_gap = Meters::from_engine(velocity_radius - envelope);
    assert!(
        (hull_gap - HULL_CLEARANCE).abs() < RADIUS_TOLERANCE,
        "shell range ({hull}): the velocity shell stands {hull_gap:?} outside the \
         {:?} envelope, not {HULL_CLEARANCE:?}",
        Meters::from_engine(envelope)
    );
    let shell_gap = Meters::from_engine(gravity_radius - velocity_radius);
    assert!(
        (shell_gap - SHELL_SEPARATION).abs() < RADIUS_TOLERANCE,
        "shell range ({hull}): the gravity shell stands {shell_gap:?} outside the velocity \
         shell, not {SHELL_SEPARATION:?}"
    );
    // Flat space: the gravity shell keeps its own rule and stays down, sized
    // but not drawn.
    assert_eq!(
        gravity_visibility,
        Visibility::Hidden,
        "shell range ({hull}): the gravity shell is up with no well to point at"
    );

    nova_probe::probe_marker(
        world,
        "outcome: both shells enclose the live hull",
        serde_json::json!({
            "hull": hull,
            "envelope_m": Meters::from_engine(envelope).get(),
            "velocity_radius_m": Meters::from_engine(velocity_radius).get(),
            "gravity_radius_m": Meters::from_engine(gravity_radius).get(),
        }),
    );
    info!(
        "shell range ({hull}): envelope {:.1} m, shells {:.1} m / {:.1} m, both on the COM",
        Meters::from_engine(envelope).get(),
        Meters::from_engine(velocity_radius).get(),
        Meters::from_engine(gravity_radius).get(),
    );
}

/// Invariant 2: both chips' near edge is [`CHIP_GAP_PX`] past the projected
/// outer shell, on their authored rows.
///
/// Asserted on the PLACEMENT, not on the drawn node: a chip whose anchor
/// projects off the viewport is hidden by the widget's own off-screen rule, and
/// on the biggest hull in the fleet the outer shell reaches the edge of the
/// frame under the stock chase rig. Where the chip was PUT is the contract this
/// task changed.
#[cfg(feature = "debug")]
fn assert_chips_clear(world: &mut World, hull: &str) {
    let ship = player_root(world);
    let com = live_com(world, ship);
    let (gravity_radius, ..) = shell_state(world, VelocityHudSource::Gravity);

    let (camera_pose, camera) = world
        .query_filtered::<(&GlobalTransform, &Camera), With<ScreenIndicatorCamera>>()
        .iter(world)
        .next()
        .map(|(pose, camera)| (*pose, camera.clone()))
        .expect("shell range: no ScreenIndicatorCamera");
    let project = |point: Vec3| {
        camera
            .world_to_viewport(&camera_pose, point)
            .expect("shell range: the hull does not project onto the viewport")
    };
    let centre_px = project(com);
    let edge_px = project(com + camera_pose.right() * gravity_radius);

    let speed = chip_placement::<SpeedChipUIMarker>(world, "speed");
    let mode = chip_placement::<ModeChipUIMarker>(world, "mode");
    for (what, (offset, half_width), row) in
        [("speed", speed, SPEED_ROW_PX), ("mode", mode, MODE_ROW_PX)]
    {
        let near_edge = centre_px.x + offset.x - half_width;
        let gap = near_edge - edge_px.x;
        assert!(
            gap >= CHIP_GAP_PX - CHIP_TOLERANCE_PX,
            "shell range ({hull}): the {what} chip's near edge is {gap:.1} px past the \
             projected shell edge, not {CHIP_GAP_PX:.0} px"
        );
        assert_eq!(
            offset.y, row,
            "shell range ({hull}): the {what} chip left its authored row"
        );
    }

    nova_probe::probe_marker(
        world,
        "outcome: the flight chips clear the outer shell",
        serde_json::json!({
            "hull": hull,
            "shell_edge_px": edge_px.x - centre_px.x,
            "speed_offset_px": speed.0.x,
            "mode_offset_px": mode.0.x,
        }),
    );
    info!(
        "shell range ({hull}): shell edge {:.0} px out, chips at {:.0} / {:.0} px",
        edge_px.x - centre_px.x,
        speed.0.x,
        mode.0.x,
    );
}

/// One chip's screen offset and half its laid-out width (logical px).
#[cfg(feature = "debug")]
fn chip_placement<M: Component>(world: &mut World, what: &str) -> (Vec2, f32) {
    world
        .query_filtered::<(&ScreenIndicatorOffset, &bevy::ui::ComputedNode), With<M>>()
        .iter(world)
        .next()
        .map(|(offset, node)| (**offset, node.size().x * node.inverse_scale_factor() / 2.0))
        .unwrap_or_else(|| panic!("shell range: no {what} chip"))
}

/// Cut the live sections that DECIDE the envelope - the furthest from the
/// centre of mass, and everything tied with them - and record what the shells
/// measured just before the cut.
#[cfg(feature = "debug")]
fn sever_envelope_sections(world: &mut World) {
    let ship = player_root(world);
    let envelope = envelope_of(world, ship);
    let (velocity_radius, ..) = shell_state(world, VelocityHudSource::Velocity);
    let (gravity_radius, ..) = shell_state(world, VelocityHudSource::Gravity);
    world.resource_mut::<ShellProbe>().before = Some((envelope, velocity_radius, gravity_radius));

    let center_of_mass = world
        .get::<avian3d::prelude::ComputedCenterOfMass>(ship)
        .copied()
        .expect("shell range: the hull has a centre of mass")
        .0;
    // The same measurement the publisher makes, in the root's own frame: a
    // section's furthest collider point from the centre of mass.
    let mut reach: Vec<(Entity, f32)> = world
        .query_filtered::<(Entity, &Transform, Option<&SectionCollider>, &ChildOf), (
            With<SectionMarker>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .filter(|(.., &ChildOf(parent))| parent == ship)
        .map(|(entity, transform, collider, _)| {
            let collider = collider.copied().unwrap_or_default();
            (
                entity,
                collider.furthest_distance(transform.translation, transform.rotation, center_of_mass),
            )
        })
        .collect();
    reach.sort_by(|a, b| b.1.total_cmp(&a.1));
    let furthest = reach.first().map(|(_, reach)| *reach).unwrap_or_default();
    let doomed: Vec<Entity> = reach
        .into_iter()
        .filter(|(_, reach)| *reach >= furthest - ENVELOPE_TIE.to_engine())
        .map(|(entity, _)| entity)
        .collect();

    info!(
        "shell range: severing {} sections at the {:.1} m envelope",
        doomed.len(),
        Meters::from_engine(furthest).get(),
    );
    for entity in doomed {
        let Some(health) = world.get::<Health>(entity).map(|health| health.current) else {
            continue;
        };
        world.trigger(HealthApplyDamage {
            entity,
            source: None,
            amount: health * 2.0,
        });
    }
}

/// Invariants 3 and 4: the envelope and both radii followed the hull down, and
/// the shells are still nested around it.
#[cfg(feature = "debug")]
fn assert_shells_shrank(world: &mut World, hull: &str) {
    let (before_envelope, before_velocity, before_gravity) = world
        .resource::<ShellProbe>()
        .before
        .expect("shell range: the cut recorded nothing");
    let ship = player_root(world);
    let envelope = envelope_of(world, ship);
    let (velocity_radius, ..) = shell_state(world, VelocityHudSource::Velocity);
    let (gravity_radius, ..) = shell_state(world, VelocityHudSource::Gravity);

    assert!(
        envelope < before_envelope,
        "shell range ({hull}): the envelope did not come down ({envelope} vs {before_envelope})"
    );
    assert!(
        velocity_radius < before_velocity && gravity_radius < before_gravity,
        "shell range ({hull}): the shells did not follow the hull down \
         ({velocity_radius} / {gravity_radius} vs {before_velocity} / {before_gravity})"
    );
    nova_probe::probe_marker(
        world,
        "outcome: severing the hull shrinks its shells",
        serde_json::json!({
            "hull": hull,
            "envelope_before_m": Meters::from_engine(before_envelope).get(),
            "envelope_after_m": Meters::from_engine(envelope).get(),
        }),
    );

    assert!(
        gravity_radius > velocity_radius,
        "shell range ({hull}): the gravity shell fell inside the velocity shell \
         ({gravity_radius} vs {velocity_radius})"
    );
    assert!(
        velocity_radius >= envelope,
        "shell range ({hull}): the velocity shell fell inside the hull \
         ({velocity_radius} vs {envelope})"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the shells stay nested through the shrink",
        serde_json::json!({
            "hull": hull,
            "velocity_radius_m": Meters::from_engine(velocity_radius).get(),
            "gravity_radius_m": Meters::from_engine(gravity_radius).get(),
        }),
    );
    info!(
        "shell range ({hull}): envelope {:.1} -> {:.1} m, shells still nested",
        Meters::from_engine(before_envelope).get(),
        Meters::from_engine(envelope).get(),
    );
}
