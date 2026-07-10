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
//!   ring (velocity-sphere visual family) plus a `r | v_circ` chip on the
//!   ring point nearest the ship.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{holo_instruments::HoloAssets, screen_indicator::prelude::*, NAV_CYAN};
use crate::{
    flight::{orbit_ring_point, prelude::*},
    gravity::prelude::*,
    input::prelude::*,
};

pub mod prelude {
    pub use super::{
        maneuver_instruments_hud, ManeuverInstrumentsHudConfig, ManeuverInstrumentsHudMarker,
        ManeuverInstrumentsPlugin, OrbitRingMarker,
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

/// Marker for the orbit-ring chip.
#[derive(Component, Debug, Clone, Reflect)]
struct OrbitChipUIMarker;

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

/// UI bundle: one indicator layer with the three chips. The holo ring is
/// not part of this layer - it is a world-space entity owned by
/// [`sync_orbit_ring`].
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
                Name::new("OrbitChipUI"),
                OrbitChipUIMarker,
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

        app.register_type::<OrbitRingMarker>();

        app.add_systems(
            Update,
            (
                (
                    drive_destination_readout,
                    drive_flip_marker,
                    drive_orbit_chip,
                )
                    .before(ScreenIndicatorSystems),
                sync_orbit_ring,
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
                **text = format!(
                    "{eta}{:4.1} u/s | {:5.0}m",
                    telemetry.closing_speed, telemetry.distance
                );
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

/// `r <radius> | <v_circ> u/s` on the planned ring's point nearest the
/// ship, while an ORBIT plan is engaged and its well still exists.
fn drive_orbit_chip(
    q_hud: Query<&ManeuverInstrumentsShipEntity, With<ManeuverInstrumentsHudMarker>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text, &ChildOf), With<OrbitChipUIMarker>>,
    q_ship: Query<(&Position, &Autopilot)>,
    q_well: Query<(&Position, &GravityWell)>,
) {
    for (mut anchor, mut text, &ChildOf(parent)) in &mut q_ui {
        let Ok(ship) = q_hud.get(parent) else {
            continue;
        };

        let ring = q_ship.get(**ship).ok().and_then(|(position, autopilot)| {
            let AutopilotAction::Orbit {
                well,
                plan: Some(plan),
            } = autopilot.action
            else {
                return None;
            };
            let (well_position, well_data) = q_well.get(well).ok()?;
            let point = **well_position + orbit_ring_point(**position - **well_position, &plan);
            Some((
                point,
                plan.radius,
                circular_orbit_speed(well_data.mu, plan.radius),
            ))
        });

        match ring {
            Some((point, radius, v_circ)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Point(point));
                **text = format!("r {radius:3.0} | {v_circ:3.1} u/s");
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
        assert_eq!(text_of(&world, readout), "ETA  18s | 12.0 u/s |   300m");
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
    fn orbit_ring_and_chip_live_and_die_with_the_plan() {
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
        let (_, _, chip) = spawn_instruments(&mut world, ship);

        world.run_system_once(sync_orbit_ring).unwrap();
        world.run_system_once(drive_orbit_chip).unwrap();

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
        assert!(text_of(&world, chip).starts_with("r  50 | 4.9"));

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

        // Breakout: the maneuver ends, the holo and the chip go with it.
        world.entity_mut(ship).remove::<Autopilot>();
        world.run_system_once(sync_orbit_ring).unwrap();
        world.run_system_once(drive_orbit_chip).unwrap();
        assert!(
            !world.entities().contains(ring_entity),
            "the ring dies with the maneuver"
        );
        assert_eq!(anchor_of(&world, chip), None);
    }
}
