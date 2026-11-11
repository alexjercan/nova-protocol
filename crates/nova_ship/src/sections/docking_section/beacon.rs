//! The beacon on every docking port of a station: one point light per port,
//! cold white and breathing slowly, amber while that port holds a dock, dark
//! on a dead station.
//!
//! Long-lived, like the railgun charge glow: a beacon is spawned with its
//! port and stays, so it is NOT a transient light and holds no slot of the
//! [`GraphicsBudget::transient_lights`] cap. The cap's ZERO is the gate: a
//! tier that lights no flash lights (Low, or Custom with flash lights Off)
//! spawns no beacon, and a tier that drops to zero mid-session takes the
//! lit ones down; a rise lights every bare port again.
//!
//! A port is lit once ([`StationBeaconPort`] records it), so a station the
//! world thaws, whose ports are fresh entities, lights fresh beacons and a
//! marker re-added to a standing hull adds none. A dead port's art goes to
//! its wreck piece with the beacon on it; the pulse despawns a beacon whose
//! parent is no longer a live port the next frame, before any freeze could
//! record it.

use bevy::prelude::*;
use nova_gameplay::prelude::{DerelictShipMarker, GraphicsBudget, StationMarker};

use super::{DockedPort, DockingSectionMarker, PORT_FACE};

/// Peak brightness of a beacon, in lumens: three clamp blinks, enough to
/// read on a collar sitting in the station's own shadow.
const BEACON_LUMENS: f32 = 60_000.0;

/// How far a beacon reaches, in world units (600 m): the collar, the plating
/// around it and a hull on its final approach.
const BEACON_RANGE: f32 = 60.0;

/// How far past the retracted face the beacon hangs, in world units (10 m).
/// At the port's origin the light sits inside the cell and lights nothing:
/// every face of the port and of its neighbours points away from it. Out
/// past the face it lands on the collar and the plating around it.
const BEACON_STANDOFF: f32 = 1.0;

/// One breath of the pulse, in virtual seconds.
const BEACON_PULSE_SECS: f32 = 3.0;

/// The floor of the pulse, as a share of [`BEACON_LUMENS`]: a beacon never
/// goes out between breaths.
const BEACON_PULSE_FLOOR: f32 = 0.45;

/// A free port's beacon: cold white, the clamp blink's register.
const BEACON_FREE_COLOR: Color = Color::srgb(0.78, 0.88, 1.0);

/// A held port's beacon: amber, the HUD's own warning tone.
const BEACON_HELD_COLOR: Color = Color::srgb(1.0, 0.62, 0.18);

/// A station port's beacon light, a child of the port.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct StationBeacon {
    /// Where in the pulse this beacon starts, in seconds, so one station's
    /// ports do not breathe in lockstep.
    pub phase: f32,
}

/// On a station port that carries its beacon.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct StationBeaconPort;

/// Whether `budget` lights beacons at all: an app with no budget renders
/// everything, as the transient-light cap reads it.
fn beacons_wanted(budget: Option<&GraphicsBudget>) -> bool {
    budget.is_none_or(|budget| budget.transient_lights > 0)
}

/// Light every bare docking port of a station hull.
pub(super) fn light_station_ports(
    budget: Option<Res<GraphicsBudget>>,
    q_ports: Query<(Entity, &ChildOf), (With<DockingSectionMarker>, Without<StationBeaconPort>)>,
    q_stations: Query<(), With<StationMarker>>,
    mut commands: Commands,
) {
    if !beacons_wanted(budget.as_deref()) {
        return;
    }
    for (port, child_of) in &q_ports {
        let station = child_of.parent();
        if !q_stations.contains(station) {
            continue;
        }
        // Spread by the port's index: enough that neighbouring ports of one
        // station start their breath apart.
        let phase = (port.index().index() % 7) as f32 * (BEACON_PULSE_SECS / 7.0);
        debug!("station_beacon: port {port} of station {station} lit, phase {phase:.2} s");
        commands.entity(port).insert(StationBeaconPort);
        commands.entity(port).with_child((
            Name::new("Station Beacon"),
            StationBeacon { phase },
            Transform::from_translation(PORT_FACE + Vec3::NEG_Z * BEACON_STANDOFF),
            PointLight {
                color: BEACON_FREE_COLOR,
                intensity: BEACON_LUMENS,
                range: BEACON_RANGE,
                radius: 0.0,
                // A beacon sits on the collar it lights; a shadow map here
                // would spend a cascade on the port occluding itself.
                shadow_maps_enabled: false,
                ..default()
            },
        ));
    }
}

/// Breathe every beacon on the virtual clock, amber while its port is held
/// and dark on a dead station; take down a beacon whose port is gone or
/// whose tier no longer lights any.
pub(super) fn pulse_station_beacons(
    time: Res<Time<Virtual>>,
    budget: Option<Res<GraphicsBudget>>,
    mut q_beacons: Query<(Entity, &StationBeacon, &ChildOf, &mut PointLight)>,
    q_ports: Query<(&ChildOf, Has<DockedPort>), With<DockingSectionMarker>>,
    q_stations: Query<Has<DerelictShipMarker>, With<StationMarker>>,
    mut commands: Commands,
) {
    let wanted = beacons_wanted(budget.as_deref());
    let now = time.elapsed_secs();
    for (beacon, StationBeacon { phase }, child_of, mut light) in &mut q_beacons {
        let port = child_of.parent();
        let Ok((port_of, held)) = q_ports.get(port) else {
            commands.entity(beacon).despawn();
            continue;
        };
        if !wanted {
            commands.entity(beacon).despawn();
            commands.entity(port).remove::<StationBeaconPort>();
            continue;
        }
        let dead = q_stations.get(port_of.parent()).unwrap_or(false);
        let breath = 0.5 + 0.5 * ((now + phase) * std::f32::consts::TAU / BEACON_PULSE_SECS).sin();
        let intensity = if dead {
            0.0
        } else {
            BEACON_LUMENS * (BEACON_PULSE_FLOOR + (1.0 - BEACON_PULSE_FLOOR) * breath)
        };
        let color = if held {
            BEACON_HELD_COLOR
        } else {
            BEACON_FREE_COLOR
        };
        if light.intensity != intensity {
            light.intensity = intensity;
        }
        if light.color != color {
            light.color = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::GraphicsQuality;

    use super::*;

    fn app(quality: GraphicsQuality) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GraphicsBudget::for_quality(quality));
        app.add_systems(Update, (light_station_ports, pulse_station_beacons).chain());
        app
    }

    /// A station hull with `ports` docking ports.
    fn station(app: &mut App, ports: usize) -> (Entity, Vec<Entity>) {
        let root = app
            .world_mut()
            .spawn((StationMarker, Transform::default()))
            .id();
        let ports = (0..ports)
            .map(|_| {
                app.world_mut()
                    .spawn((ChildOf(root), DockingSectionMarker, Transform::default()))
                    .id()
            })
            .collect();
        (root, ports)
    }

    /// Every beacon with the port it hangs on.
    fn beacons(app: &mut App) -> Vec<(Entity, PointLight)> {
        app.world_mut()
            .query_filtered::<(&ChildOf, &PointLight), With<StationBeacon>>()
            .iter(app.world())
            .map(|(child_of, light)| (child_of.parent(), *light))
            .collect()
    }

    #[test]
    fn a_beacon_hangs_out_past_the_port_face_not_inside_the_cell() {
        let mut app = app(GraphicsQuality::High);
        station(&mut app, 1);
        app.update();
        app.update();
        let hung: Vec<f32> = app
            .world_mut()
            .query_filtered::<&Transform, With<StationBeacon>>()
            .iter(app.world())
            .map(|transform| transform.translation.z)
            .collect();
        assert_eq!(hung.len(), 1);
        assert!(
            hung[0] < PORT_FACE.z,
            "the beacon must hang past the face at z {} to light the collar, sat at z {}",
            PORT_FACE.z,
            hung[0]
        );
    }

    #[test]
    fn a_station_with_two_ports_gets_one_beacon_per_port_on_high() {
        let mut app = app(GraphicsQuality::High);
        let (_, ports) = station(&mut app, 2);
        app.update();
        app.update();
        let mut lit: Vec<Entity> = beacons(&mut app)
            .into_iter()
            .map(|(port, _)| port)
            .collect();
        lit.sort();
        let mut expected = ports.clone();
        expected.sort();
        assert_eq!(lit, expected);
    }

    #[test]
    fn a_hull_that_is_not_a_station_gets_no_beacon() {
        let mut app = app(GraphicsQuality::High);
        let root = app.world_mut().spawn(Transform::default()).id();
        app.world_mut()
            .spawn((ChildOf(root), DockingSectionMarker, Transform::default()));
        app.update();
        app.update();
        assert!(beacons(&mut app).is_empty());
    }

    #[test]
    fn low_lights_no_beacon_and_a_tier_change_applies_live() {
        let mut app = app(GraphicsQuality::Low);
        station(&mut app, 2);
        app.update();
        app.update();
        assert!(beacons(&mut app).is_empty(), "Low lights nothing");

        app.insert_resource(GraphicsBudget::for_quality(GraphicsQuality::Medium));
        app.update();
        app.update();
        assert_eq!(beacons(&mut app).len(), 2, "a rise lights every port");

        app.insert_resource(GraphicsBudget::for_quality(GraphicsQuality::Low));
        app.update();
        app.update();
        assert!(
            beacons(&mut app).is_empty(),
            "a drop to zero takes them down"
        );
    }

    #[test]
    fn a_held_port_burns_amber_and_returns_to_white_on_release() {
        let mut app = app(GraphicsQuality::High);
        let (_, ports) = station(&mut app, 1);
        app.update();
        app.update();
        assert_eq!(beacons(&mut app)[0].1.color, BEACON_FREE_COLOR);

        app.world_mut()
            .entity_mut(ports[0])
            .insert(DockedPort(Entity::PLACEHOLDER));
        app.update();
        assert_eq!(beacons(&mut app)[0].1.color, BEACON_HELD_COLOR);

        app.world_mut().entity_mut(ports[0]).remove::<DockedPort>();
        app.update();
        assert_eq!(beacons(&mut app)[0].1.color, BEACON_FREE_COLOR);
    }

    #[test]
    fn a_dead_station_keeps_its_beacons_dark() {
        let mut app = app(GraphicsQuality::High);
        let (root, _) = station(&mut app, 1);
        app.world_mut().entity_mut(root).insert(DerelictShipMarker);
        app.update();
        app.update();
        let lit = beacons(&mut app);
        assert_eq!(lit.len(), 1);
        assert_eq!(lit[0].1.intensity, 0.0);
    }

    #[test]
    fn a_marker_re_added_to_a_lit_station_adds_no_second_beacon() {
        let mut app = app(GraphicsQuality::High);
        let (root, _) = station(&mut app, 2);
        app.update();
        app.update();
        app.world_mut().entity_mut(root).insert(StationMarker);
        app.update();
        app.update();
        assert_eq!(beacons(&mut app).len(), 2);
    }

    #[test]
    fn a_beacon_whose_port_is_no_longer_a_port_is_taken_down() {
        let mut app = app(GraphicsQuality::High);
        let (_, ports) = station(&mut app, 2);
        app.update();
        app.update();
        app.world_mut()
            .entity_mut(ports[0])
            .remove::<DockingSectionMarker>();
        app.update();
        app.update();
        let lit = beacons(&mut app);
        assert_eq!(lit.len(), 1);
        assert_eq!(lit[0].0, ports[1]);
    }
}
