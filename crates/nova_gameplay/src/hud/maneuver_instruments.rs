//! The maneuver instruments (task 20260709-103454, spike
//! docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md):
//! the engaged maneuver made visible in the hybrid language the spike
//! decided - projected chips for numbers, world-space holo geometry for
//! spatial facts.
//!
//! - **Destination readout**: a text chip below the GOTO destination
//!   marker with ETA, closing speed and distance, fed by the physics-side
//!   [`ManeuverTelemetry`] seam (the HUD computes nothing).
//! - **Flip marker**: a `FLIP <n>s` chip projected on the flight path
//!   where the arrival rule says the flip-and-burn starts.
//! - **ORBIT holo ring**: a world-space torus at the engaged orbit plan's
//!   ring (velocity-sphere visual family).
//! - **Radius spoke** (task 20260710-231926, spike
//!   docs/spikes/20260710-234019-diegetic-flight-status.md): while ORBIT
//!   is engaged, a thin holo line from the well center to the ship with
//!   the current radius as a chip at its midpoint - the in-world home of
//!   the old status line's `r` readout.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    holo_instruments::{segment_transform, HoloAssets},
    screen_indicator::prelude::*,
    NAV_CYAN,
};
use crate::{flight::prelude::*, gravity::prelude::*, input::prelude::*};

pub mod prelude {
    pub use super::{
        maneuver_instruments_hud, ManeuverInstrumentsHudConfig, ManeuverInstrumentsHudMarker,
        ManeuverInstrumentsPlugin, OrbitRingMarker, RadiusSpokeMarker,
    };
}

/// On-screen size of the small chips (px).
const CHIP_SIZE: Vec2 = Vec2::new(120.0, 16.0);

/// The destination readout line is ~28 chars at font 12 (~9 px/char);
/// size the chip for it and forbid wrapping so it stays one line.
const READOUT_SIZE: Vec2 = Vec2::new(260.0, 16.0);

/// The destination readout sits this far below the destination marker (px).
const READOUT_OFFSET: Vec2 = Vec2::new(0.0, 28.0);

/// The ring holo's tube thickness, world units - thin, an instrument line,
/// not a solid.
const RING_MINOR_RADIUS: f32 = 0.15;

#[derive(Component, Debug, Clone, Reflect)]
pub struct ManeuverInstrumentsHudMarker;

/// The ship whose maneuver these instruments show.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct ManeuverInstrumentsShipEntity(Entity);

/// Marker for the destination readout chip.
#[derive(Component, Debug, Clone, Reflect)]
struct DestinationReadoutUIMarker;

/// Marker for the flip-point chip.
#[derive(Component, Debug, Clone, Reflect)]
struct FlipMarkerUIMarker;

/// Marker for the radius-spoke chip.
#[derive(Component, Debug, Clone, Reflect)]
struct RadiusSpokeChipUIMarker;

/// The world-space holo spoke from the orbited well's center to the ship.
/// Public so the HUD teardown sweep (and tests) can find it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct RadiusSpokeMarker {
    /// The ship whose orbit radius this spoke renders.
    pub ship: Entity,
}

/// The world-space holo ring of an engaged ORBIT plan. Public so tests and
/// the future holo-expansion task can find it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct OrbitRingMarker {
    /// The ship whose plan this ring renders.
    pub ship: Entity,
    /// The plan radius the mesh was built at, to detect replans.
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct ManeuverInstrumentsHudConfig {
    pub ship: Entity,
}

/// UI bundle: one indicator layer with the three chips. The holo ring and
/// the radius spoke are not part of this layer - they are world-space
/// entities owned by [`sync_orbit_ring`] and [`sync_radius_spoke`].
pub fn maneuver_instruments_hud(config: ManeuverInstrumentsHudConfig) -> impl Bundle {
    debug!("maneuver_instruments_hud: config {:?}", config);

    let chip = |size: Vec2, offset: Vec2| {
        screen_indicator(ScreenIndicatorConfig {
            anchor: None,
            size: ScreenIndicatorSize::Fixed(size),
            offset,
            offscreen: ScreenIndicatorOffscreen::Hide,
        })
    };

    (
        Name::new("ManeuverInstrumentsHUD"),
        ManeuverInstrumentsHudMarker,
        ManeuverInstrumentsShipEntity(config.ship),
        screen_indicator_layer(),
        children![
            (
                Name::new("DestinationReadoutUI"),
                DestinationReadoutUIMarker,
                chip(READOUT_SIZE, READOUT_OFFSET),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(NAV_CYAN),
            ),
            (
                Name::new("FlipMarkerUI"),
                FlipMarkerUIMarker,
                chip(CHIP_SIZE, Vec2::ZERO),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(NAV_CYAN),
            ),
            (
                Name::new("RadiusSpokeChipUI"),
                RadiusSpokeChipUIMarker,
                chip(CHIP_SIZE, Vec2::ZERO),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(NAV_CYAN),
            ),
        ],
    )
}

#[derive(Default)]
pub struct ManeuverInstrumentsPlugin;

impl Plugin for ManeuverInstrumentsPlugin {
    fn build(&self, app: &mut App) {
        debug!("ManeuverInstrumentsPlugin: build");

        // Shared with the holo-instruments family (one material batches
        // the whole set); idempotent with HoloInstrumentsPlugin's init.
        app.init_resource::<HoloAssets>();

        app.register_type::<OrbitRingMarker>()
            .register_type::<RadiusSpokeMarker>();

        app.add_systems(
            Update,
            (
                // Drivers run in Update; the projection consumes their
                // anchors in PostUpdate by schedule order.
                (
                    drive_destination_readout,
                    drive_flip_marker,
                    drive_radius_spoke_chip,
                ),
                sync_orbit_ring,
                sync_radius_spoke,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// ETA, closing speed and distance below the destination marker, from the
/// telemetry the autopilot publishes. No telemetry (manual, STOP, ORBIT,
/// inside the standoff with the leg done) clears the anchor and the widget
/// hides the chip.
fn drive_destination_readout(
    q_hud: Query<&ManeuverInstrumentsShipEntity, With<ManeuverInstrumentsHudMarker>>,
    mut q_ui: Query<
        (&mut ScreenIndicatorAnchor, &mut Text, &ChildOf),
        With<DestinationReadoutUIMarker>,
    >,
    q_ship: Query<&ManeuverTelemetry>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        match q_ship.get(**ship) {
            Ok(telemetry) => {
                // Anchor the tracked entity when there is one, so a moving
                // GOTO target carries its caption with the (interpolated)
                // destination marker instead of the fixed-tick snapshot.
                **anchor = Some(match telemetry.goal_entity {
                    Some(entity) => ScreenIndicatorAnchorKind::Entity(entity),
                    None => ScreenIndicatorAnchorKind::Point(telemetry.goal),
                });
                let eta = match telemetry.eta {
                    Some(eta) => format!("ETA {eta:3.0}s | "),
                    None => String::new(),
                };
                // ETA and distance only: the ship's own speed chip already
                // shows the velocity, and two speed readouts in one glance
                // was playtest-flagged redundancy (task 20260711-125226,
                // same call as the orbit ring chip removal).
                **text = format!("{eta}{:5.0}m", telemetry.distance);
            }
            Err(_) => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// `FLIP <n>s` on the path point where braking starts; hidden once the
/// ship is braking (the autopilot stops predicting a flip).
fn drive_flip_marker(
    q_hud: Query<&ManeuverInstrumentsShipEntity, With<ManeuverInstrumentsHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text, &ChildOf), With<FlipMarkerUIMarker>>,
    q_ship: Query<&ManeuverTelemetry>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        let flip = q_ship
            .get(**ship)
            .ok()
            .and_then(|t| t.flip_point.zip(t.seconds_to_flip));
        match flip {
            Some((point, seconds)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Point(point));
                **text = format!("FLIP {seconds:3.0}s");
            }
            None => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// Own the world-space holo ring: spawn it when the player's ORBIT plan
/// appears, keep it on the well and on the plan (rebuild on replan),
/// despawn it when the maneuver or the well ends. Asset access is plain
/// `Assets<_>` so the lifecycle runs headless in tests.
fn sync_orbit_ring(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<HoloAssets>,
    q_ship: Query<(Entity, &Autopilot), With<PlayerSpaceshipMarker>>,
    q_well: Query<&Position, With<GravityWell>>,
    mut q_ring: Query<(Entity, &OrbitRingMarker, &mut Transform)>,
) {
    // The player's engaged plan, if any (one player, at most one ring).
    let engaged = q_ship.iter().find_map(|(ship, autopilot)| {
        let AutopilotAction::Orbit {
            well,
            plan: Some(plan),
        } = autopilot.action
        else {
            return None;
        };
        let well_position = q_well.get(well).ok()?;
        Some((ship, plan, **well_position))
    });

    match engaged {
        Some((ship, plan, well_position)) => {
            let rotation = Quat::from_rotation_arc(Vec3::Y, plan.normal);
            let mut found = false;
            for (entity, marker, mut transform) in &mut q_ring {
                if marker.ship != ship || (marker.radius - plan.radius).abs() > f32::EPSILON {
                    // A stale ring (other ship, or a replanned radius):
                    // rebuild from scratch.
                    commands.entity(entity).despawn();
                    continue;
                }
                found = true;
                // Guard the write: an unconditional Mut deref would dirty
                // change detection and re-propagate the ring every frame.
                if transform.translation != well_position || transform.rotation != rotation {
                    transform.translation = well_position;
                    transform.rotation = rotation;
                }
            }
            if !found {
                commands.spawn((
                    Name::new("OrbitRingHolo"),
                    OrbitRingMarker {
                        ship,
                        radius: plan.radius,
                    },
                    Mesh3d(meshes.add(Torus::new(
                        plan.radius - RING_MINOR_RADIUS,
                        plan.radius + RING_MINOR_RADIUS,
                    ))),
                    MeshMaterial3d(assets.material(&mut materials)),
                    Transform::from_translation(well_position).with_rotation(rotation),
                    Visibility::Visible,
                ));
            }
        }
        None => {
            for (entity, _, _) in &q_ring {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// `r <radius>` at the spoke's midpoint, while ORBIT is engaged and the
/// well exists. The current radius, unlike the ring chip's planned one -
/// the pair converges as the insertion settles.
fn drive_radius_spoke_chip(
    q_hud: Query<&ManeuverInstrumentsShipEntity, With<ManeuverInstrumentsHudMarker>>,
    mut q_ui: Query<
        (&mut ScreenIndicatorAnchor, &mut Text, &ChildOf),
        With<RadiusSpokeChipUIMarker>,
    >,
    // Ship pose on the RENDER clock (eased root Transform), not raw avian
    // Position: the chip rides the midpoint of a line whose ship end the
    // player SEES, and at speed the raw pose leads the rendered hull by up
    // to a tick (task 20260710-231928). Wells are static, so their raw
    // Position is identical either way.
    q_ship: Query<(&Transform, &Autopilot)>,
    q_well: Query<&Position, With<GravityWell>>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        let spoke = q_ship.get(**ship).ok().and_then(|(transform, autopilot)| {
            let AutopilotAction::Orbit { well, .. } = autopilot.action else {
                return None;
            };
            let well_position = q_well.get(well).ok()?;
            Some((**well_position, transform.translation))
        });

        match spoke {
            Some((well_position, ship_position)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Point(
                    (well_position + ship_position) * 0.5,
                ));
                **text = format!("r {:4.0}", well_position.distance(ship_position));
            }
            None => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// Own the radius spoke: a thin holo line from the well center to the ship
/// while the player's ORBIT is engaged (with or without a plan - the
/// current radius exists the moment the verb does), stretched every frame
/// (both ends move), despawned when the maneuver or the well ends.
fn sync_radius_spoke(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<HoloAssets>,
    // The spoke's ship end must meet the RENDERED hull: eased root
    // Transform, not raw Position (see drive_radius_spoke_chip).
    q_ship: Query<
        (Entity, &Transform, &Autopilot),
        (With<PlayerSpaceshipMarker>, Without<RadiusSpokeMarker>),
    >,
    q_well: Query<&Position, With<GravityWell>>,
    mut q_spoke: Query<(Entity, &RadiusSpokeMarker, &mut Transform)>,
) {
    let engaged = q_ship.iter().find_map(|(ship, transform, autopilot)| {
        let AutopilotAction::Orbit { well, .. } = autopilot.action else {
            return None;
        };
        let well_position = q_well.get(well).ok()?;
        Some((ship, **well_position, transform.translation))
    });

    let Some((ship, well_position, ship_position)) = engaged else {
        for (entity, _, _) in &q_spoke {
            commands.entity(entity).despawn();
        }
        return;
    };

    let transform = segment_transform(well_position, ship_position);
    let mut found = false;
    for (entity, spoke, mut spoke_transform) in &mut q_spoke {
        if spoke.ship != ship {
            commands.entity(entity).despawn();
            continue;
        }
        found = true;
        *spoke_transform = transform;
    }
    if !found {
        commands.spawn((
            Name::new("RadiusSpokeHolo"),
            RadiusSpokeMarker { ship },
            Mesh3d(assets.segment_mesh(&mut meshes)),
            MeshMaterial3d(assets.material(&mut materials)),
            transform,
            Visibility::Visible,
        ));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::sections::prelude::*;

    fn spawn_instruments(world: &mut World, ship: Entity) -> (Entity, Entity, Entity) {
        let layer = world
            .spawn(maneuver_instruments_hud(ManeuverInstrumentsHudConfig {
                ship,
            }))
            .id();
        let children = world.entity(layer).get::<Children>().unwrap();
        (children[0], children[1], children[2])
    }

    fn anchor_of(world: &World, entity: Entity) -> Option<ScreenIndicatorAnchorKind> {
        **world.entity(entity).get::<ScreenIndicatorAnchor>().unwrap()
    }

    fn text_of(world: &World, entity: Entity) -> String {
        world.entity(entity).get::<Text>().unwrap().0.clone()
    }

    #[test]
    fn destination_readout_and_flip_marker_follow_the_telemetry() {
        let mut world = World::new();
        let ship = world
            .spawn(ManeuverTelemetry {
                goal: Vec3::new(0.0, 0.0, -300.0),
                goal_entity: None,
                distance: 300.0,
                closing_speed: 12.0,
                brake_accel: 10.0,
                flip_point: Some(Vec3::new(0.0, 0.0, -240.0)),
                seconds_to_flip: Some(15.0),
                eta: Some(18.0),
            })
            .id();
        let (readout, flip, _) = spawn_instruments(&mut world, ship);

        world.run_system_once(drive_destination_readout).unwrap();
        world.run_system_once(drive_flip_marker).unwrap();

        assert_eq!(
            anchor_of(&world, readout),
            Some(ScreenIndicatorAnchorKind::Point(Vec3::new(
                0.0, 0.0, -300.0
            )))
        );
        assert_eq!(text_of(&world, readout), "ETA  18s |   300m");
        assert_eq!(
            anchor_of(&world, flip),
            Some(ScreenIndicatorAnchorKind::Point(Vec3::new(
                0.0, 0.0, -240.0
            )))
        );
        assert_eq!(text_of(&world, flip), "FLIP  15s");

        // The leg ends: telemetry gone, chips clear and hide.
        world.entity_mut(ship).remove::<ManeuverTelemetry>();
        world.run_system_once(drive_destination_readout).unwrap();
        world.run_system_once(drive_flip_marker).unwrap();
        assert_eq!(anchor_of(&world, readout), None);
        assert!(text_of(&world, readout).is_empty());
        assert_eq!(anchor_of(&world, flip), None);
    }

    #[test]
    fn flip_marker_hides_once_braking() {
        let mut world = World::new();
        let ship = world
            .spawn(ManeuverTelemetry {
                goal: Vec3::new(0.0, 0.0, -100.0),
                goal_entity: None,
                distance: 100.0,
                closing_speed: 12.0,
                brake_accel: 10.0,
                flip_point: None,
                seconds_to_flip: None,
                eta: Some(16.0),
            })
            .id();
        let (readout, flip, _) = spawn_instruments(&mut world, ship);

        world.run_system_once(drive_destination_readout).unwrap();
        world.run_system_once(drive_flip_marker).unwrap();

        assert!(anchor_of(&world, readout).is_some(), "readout stays");
        assert_eq!(anchor_of(&world, flip), None, "no flip while braking");
    }

    #[test]
    fn orbit_ring_lives_and_dies_with_the_plan() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<HoloAssets>();

        let gravity = GravitySettings::default();
        let well = world
            .spawn((
                Position(Vec3::ZERO),
                GravityWell::from_surface_gravity(3.0, 20.0, &gravity),
            ))
            .id();
        let plan = OrbitPlan {
            radius: 50.0,
            normal: Vec3::Z,
        };
        let ship = world
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                Position(Vec3::new(50.0, 0.0, 0.0)),
                Autopilot::engage(AutopilotAction::Orbit {
                    well,
                    plan: Some(plan),
                }),
            ))
            .id();
        world.run_system_once(sync_orbit_ring).unwrap();

        let (ring_entity, marker, transform) = world
            .query::<(Entity, &OrbitRingMarker, &Transform)>()
            .single(&world)
            .expect("an engaged plan spawns exactly one ring");
        assert_eq!(marker.radius, 50.0);
        assert_eq!(transform.translation, Vec3::ZERO);
        let up = transform.rotation.mul_vec3(Vec3::Y);
        assert!(
            (up - Vec3::Z).length() < 1e-5,
            "ring plane matches the plan"
        );

        // Replanning (re-engage picked a different ring) rebuilds the holo.
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: Some(OrbitPlan {
                    radius: 80.0,
                    normal: Vec3::Z,
                }),
            }));
        world.run_system_once(sync_orbit_ring).unwrap();
        // The stale ring is despawned in the same pass; the rebuilt one
        // lands next pass (commands apply between runs).
        world.run_system_once(sync_orbit_ring).unwrap();
        assert!(
            !world.entities().contains(ring_entity),
            "a replanned radius rebuilds the ring"
        );
        let (ring_entity, marker, _) = world
            .query::<(Entity, &OrbitRingMarker, &Transform)>()
            .single(&world)
            .expect("one rebuilt ring");
        assert_eq!(marker.radius, 80.0);

        // Breakout: the maneuver ends, the holo goes with it.
        world.entity_mut(ship).remove::<Autopilot>();
        world.run_system_once(sync_orbit_ring).unwrap();
        assert!(
            !world.entities().contains(ring_entity),
            "the ring dies with the maneuver"
        );
    }

    #[test]
    fn radius_spoke_and_chip_track_the_engaged_orbit() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<HoloAssets>();

        let gravity = GravitySettings::default();
        let well = world
            .spawn((
                Position(Vec3::ZERO),
                GravityWell::from_surface_gravity(3.0, 20.0, &gravity),
            ))
            .id();
        // Plan still None (insertion being solved): the current radius
        // exists the moment the verb does, so the spoke is already up.
        let ship = world
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                Transform::from_xyz(60.0, 0.0, 0.0),
                Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
            ))
            .id();
        let (_, _, chip) = spawn_instruments(&mut world, ship);

        world.run_system_once(sync_radius_spoke).unwrap();
        world.run_system_once(drive_radius_spoke_chip).unwrap();

        let (_, transform) = world
            .query::<(&RadiusSpokeMarker, &Transform)>()
            .single(&world)
            .expect("an engaged ORBIT spawns exactly one spoke");
        // The unit cylinder is stretched over well -> ship: centered at the
        // midpoint, scaled to the current radius.
        assert_eq!(transform.translation, Vec3::new(30.0, 0.0, 0.0));
        assert!((transform.scale.y - 60.0).abs() < 1e-4);
        assert_eq!(
            anchor_of(&world, chip),
            Some(ScreenIndicatorAnchorKind::Point(Vec3::new(30.0, 0.0, 0.0)))
        );
        assert_eq!(text_of(&world, chip), "r   60");

        // The ship spirals in: both ends of the spoke follow.
        world
            .entity_mut(ship)
            .insert(Transform::from_xyz(0.0, 0.0, 40.0));
        world.run_system_once(sync_radius_spoke).unwrap();
        world.run_system_once(drive_radius_spoke_chip).unwrap();
        let (_, transform) = world
            .query::<(&RadiusSpokeMarker, &Transform)>()
            .single(&world)
            .expect("still one spoke");
        assert_eq!(transform.translation, Vec3::new(0.0, 0.0, 20.0));
        assert!((transform.scale.y - 40.0).abs() < 1e-4);
        assert_eq!(text_of(&world, chip), "r   40");

        // Breakout: the spoke and the chip die with the maneuver.
        world.entity_mut(ship).remove::<Autopilot>();
        world.run_system_once(sync_radius_spoke).unwrap();
        world.run_system_once(drive_radius_spoke_chip).unwrap();
        assert_eq!(
            world.query::<&RadiusSpokeMarker>().iter(&world).count(),
            0,
            "the spoke dies with the maneuver"
        );
        assert_eq!(anchor_of(&world, chip), None);
        assert!(text_of(&world, chip).is_empty());

        // Re-engage, then destroy the well mid-orbit (rocks are
        // destructible): the spoke and the chip must go the same way.
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        world.run_system_once(sync_radius_spoke).unwrap();
        assert_eq!(
            world.query::<&RadiusSpokeMarker>().iter(&world).count(),
            1,
            "re-engaging rebuilds the spoke"
        );
        world.despawn(well);
        world.run_system_once(sync_radius_spoke).unwrap();
        world.run_system_once(drive_radius_spoke_chip).unwrap();
        assert_eq!(
            world.query::<&RadiusSpokeMarker>().iter(&world).count(),
            0,
            "the spoke dies with the well"
        );
        assert_eq!(anchor_of(&world, chip), None);
        assert!(text_of(&world, chip).is_empty());
    }
}
