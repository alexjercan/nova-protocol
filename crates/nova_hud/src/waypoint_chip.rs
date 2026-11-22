//! The waypoint chip: one nav chip for the player's chart GOTO, with the
//! target's chart code and the live distance to where the leg flies.
//!
//! A chart GOTO can fly to a station or a cell the window does not hold, so
//! no entity stands where the chip points. Before the hand-off the chip
//! anchors the `GotoPos` mark as a point; after it, the live station. The
//! shape is the beacon chip's ([`anchored_chip`](super::anchored_chip)), in
//! nav cyan, and it clamps to the viewport edge with its chevron the same
//! way, behind the camera too.
//!
//! The chip draws no line. The only line to a GOTO is the ship's
//! `FlightPrediction` (the holo ribbon), which shows nothing until the
//! prediction lands.
//!
//! Chrome tier: a waypoint is guidance, not a flight instrument.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::hud::{ChipText, ChipTone};

use super::{anchored_chip::prelude::*, screen_indicator::prelude::*, NAV_CYAN};

/// The waypoint chip layer marker and `WaypointChipHudPlugin`.
pub mod prelude {
    pub use super::{WaypointChipHudMarker, WaypointChipHudPlugin};
}

/// How the chip stands off its anchor: the beacon chip's clearance, so a
/// station anchor pushes the chip clear of its hull and a point mark floats
/// it at the floor.
const CHIP_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: Vec2::NEG_Y,
    gap_px: 12.0,
    min_px: 28.0,
};

/// The chevron: the beacon chip's, in nav cyan.
const CHEVRON: AnchoredChipChevron = AnchoredChipChevron {
    arrow_px: 16.0,
    stroke_len_px: 11.0,
    stroke_thick_px: 2.0,
    color: NAV_CYAN,
};

const LABEL_FONT_PX: f32 = 12.0;

/// Marker for the waypoint chip layer. Its [`AnchoredChipTarget`] is the
/// player ship that wears the [`Waypoint`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct WaypointChipHudMarker;

/// UI bundle for the chip layer of `ship`'s waypoint. Born with no anchor:
/// [`drive_waypoint_chip`] writes it from the autopilot.
fn waypoint_chip_hud(ship: Entity) -> impl Bundle {
    (
        Name::new("WaypointChipHUD"),
        WaypointChipHudMarker,
        anchored_chip_layer(ship),
        children![(
            Name::new("WaypointChipUI"),
            anchored_chip_node(
                None,
                ChipTone::Readout,
                CHIP_CLEARANCE,
                ScreenLabelType::Waypoint,
            ),
            children![
                (
                    Name::new("WaypointChipLabel"),
                    anchored_chip_label(
                        LABEL_FONT_PX,
                        Color::NONE,
                        ChipText::value(ChipTone::Readout),
                    ),
                ),
                (
                    Name::new("WaypointChipArrow"),
                    anchored_chip_chevron(CHEVRON, ()),
                ),
            ],
        )],
    )
}

/// Draws the waypoint chip while the player ship wears a [`Waypoint`]: the
/// layer spawns with the waypoint and comes down with it, and
/// [`drive_waypoint_chip`] runs in Update within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct WaypointChipHudPlugin;

impl Plugin for WaypointChipHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("WaypointChipHudPlugin: build");

        app.add_observer(setup_waypoint_chip);
        app.add_observer(despawn_anchored_chips::<Waypoint, WaypointChipHudMarker>);
        app.add_systems(Update, drive_waypoint_chip.in_set(super::NovaHudSystems));
    }
}

/// A waypoint grows its chip the moment the ship wears it.
fn setup_waypoint_chip(add: On<Add<Waypoint>>, mut commands: Commands) {
    commands.spawn(waypoint_chip_hud(add.entity));
}

/// Anchor the chip where the leg flies, a `GotoPos` mark as a point or the
/// `Goto` target as an entity, and write the chart code and the distance.
/// A leg that is neither, or a code the chart does not hold, hides the chip:
/// the waypoint is gone by the next world pass.
fn drive_waypoint_chip(
    chart: Option<Res<Chart>>,
    q_ship: Query<(&Waypoint, Option<&Autopilot>, &GlobalTransform)>,
    q_target: Query<&GlobalTransform>,
    q_chips: Query<&AnchoredChipTarget, With<WaypointChipHudMarker>>,
    mut q_nodes: Query<
        (&mut ScreenIndicatorAnchor, &ChildOf, &Children),
        With<AnchoredChipNodeMarker>,
    >,
    mut q_labels: Query<&mut Text, With<AnchoredChipLabelMarker>>,
) {
    for (mut anchor, ChildOf(layer), children) in &mut q_nodes {
        let Ok(ship) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((waypoint, autopilot, ship_transform)) = q_ship.get(**ship) else {
            continue;
        };
        let leg = autopilot.and_then(|autopilot| match autopilot.action {
            AutopilotAction::GotoPos { position } => {
                Some((ScreenIndicatorAnchorKind::Point(position), position))
            }
            AutopilotAction::Goto { target } => q_target
                .get(target)
                .ok()
                .map(|at| (ScreenIndicatorAnchorKind::Entity(target), at.translation())),
            _ => None,
        });
        let code = chart
            .as_deref()
            .and_then(|chart| waypoint.target.chart_code(chart));
        let (wanted, line) = match (leg, code) {
            (Some((kind, at)), Some(code)) => (
                Some(kind),
                format!(
                    "{code}  {}",
                    nova_ui::units::distance(Meters::from_engine(
                        ship_transform.translation().distance(at)
                    ))
                ),
            ),
            _ => (None, String::new()),
        };
        anchor.set_if_neq(ScreenIndicatorAnchor(wanted));
        for child in children {
            if let Ok(mut text) = q_labels.get_mut(*child) {
                if **text != line {
                    **text = line.clone();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_events::prelude::{GeneratedBodyId, SectorCoord};

    use super::*;

    fn station_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(4, 0, -1), "cluster_4_0_n1_station", 0)
    }

    /// The chip's anchor and its label.
    fn chip(world: &mut World) -> Option<(Option<ScreenIndicatorAnchorKind>, String)> {
        let anchor = world
            .query_filtered::<&ScreenIndicatorAnchor, With<AnchoredChipNodeMarker>>()
            .iter(world)
            .next()
            .map(|anchor| **anchor)?;
        let label = world
            .query_filtered::<&Text, With<AnchoredChipLabelMarker>>()
            .iter(world)
            .next()
            .map(|text| text.0.clone())?;
        Some((anchor, label))
    }

    /// The chip spawns with the waypoint and anchors the mark as a point
    /// with the chart code and the range to it; after the hand-off it
    /// anchors the live station; it comes down with the waypoint.
    #[test]
    fn the_waypoint_chip_follows_the_leg_and_leaves_with_the_waypoint() {
        let mut world = World::new();
        world.add_observer(setup_waypoint_chip);
        world.add_observer(despawn_anchored_chips::<Waypoint, WaypointChipHudMarker>);
        let mut chart = Chart::default();
        chart.find_station(&station_id(), false, false, None, 0.0);
        world.insert_resource(chart);

        let mark = Vec3::new(0.0, 0.0, -12_840.0);
        let ship = world
            .spawn((
                GlobalTransform::IDENTITY,
                Waypoint {
                    target: WaypointTargetType::Station(station_id()),
                    mark,
                },
                Autopilot::engage(AutopilotAction::GotoPos { position: mark }),
            ))
            .id();
        world.flush();
        world.run_system_once(drive_waypoint_chip).unwrap();
        assert_eq!(
            chip(&mut world),
            Some((
                Some(ScreenIndicatorAnchorKind::Point(mark)),
                "STN-1  128.40 km".to_string()
            ))
        );

        let station = world
            .spawn(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -50.0,
            )))
            .id();
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target: station }));
        world.run_system_once(drive_waypoint_chip).unwrap();
        assert_eq!(
            chip(&mut world),
            Some((
                Some(ScreenIndicatorAnchorKind::Entity(station)),
                "STN-1  500 m".to_string()
            ))
        );

        world.entity_mut(ship).remove::<Waypoint>();
        world.flush();
        assert_eq!(
            world
                .query_filtered::<(), With<WaypointChipHudMarker>>()
                .iter(&world)
                .count(),
            0
        );
    }

    /// A sector waypoint reads its cell's coordinate.
    #[test]
    fn a_sector_waypoint_chip_reads_its_cell() {
        let mut world = World::new();
        world.add_observer(setup_waypoint_chip);
        world.init_resource::<Chart>();
        let coord = SectorCoord::new(-2, 0, 3);
        let mark = Vec3::new(5_000.0, 0.0, 0.0);
        world.spawn((
            GlobalTransform::IDENTITY,
            Waypoint {
                target: WaypointTargetType::Sector(coord),
                mark,
            },
            Autopilot::engage(AutopilotAction::GotoPos { position: mark }),
        ));
        world.flush();
        world.run_system_once(drive_waypoint_chip).unwrap();
        assert_eq!(
            chip(&mut world),
            Some((
                Some(ScreenIndicatorAnchorKind::Point(mark)),
                "(-2, 0, 3)  50.00 km".to_string()
            ))
        );
    }
}
