//! What a ship is PERMITTED to do: the seven software capabilities every
//! consumer asks the ship ROOT about, so a lit hint, a firing key, the radar
//! and the autopilot can never disagree about the same hull.
//!
//! Capability is a property of the SHIP, not of any section. A controller
//! section is attitude hardware - it senses, it lags, it twists - and a hull
//! that loses every controller loses its commanded rotation, not its
//! permission to hold a lock. The two questions are asked separately here:
//! [`ShipCapabilities`] for what the ship may do, [`ship_has_attitude_authority`]
//! for whether it can still point itself.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// The root capability component and the attitude-authority query.
pub mod prelude {
    pub use super::ShipCapabilities;
}

/// What a ship is permitted to do, carried on the ship ROOT.
///
/// Every field defaults to `true`: a hull that says nothing can do everything,
/// which is what the bare rigs the examples and the section ranges fly depend
/// on. A scenario withholds a capability by authoring the `false`, and the
/// `SetShipCapability*` actions flip one field on one root at runtime.
///
/// These are SOFTWARE decisions. None of them is affected by losing a
/// controller section: a hulk with no attitude authority still knows whether
/// its point defence was configured on.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub struct ShipCapabilities {
    /// STOP: flip retrograde and burn to rest.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub stop_enabled: bool,
    /// GOTO: fly to the current nav lock and come to rest off it.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub goto_enabled: bool,
    /// ORBIT: circularize and station-keep inside a gravity well.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub orbit_enabled: bool,
    /// LOCK: the radar gesture and the combat lock it produces.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub lock_enabled: bool,
    /// RCS: the torque-free fine-adjust push about the centre of mass.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub rcs_enabled: bool,
    /// POINT DEFENCE: turrets engaging incoming ordnance on their own.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub point_defense_enabled: bool,
    /// DOCK: the explicit capture command a docking port answers. Permission
    /// only - a hull with no docking section never offers the verb whatever
    /// this says.
    #[cfg_attr(
        feature = "serde",
        serde(default = "enabled", skip_serializing_if = "is_enabled")
    )]
    pub dock_enabled: bool,
}

impl Default for ShipCapabilities {
    fn default() -> Self {
        Self {
            stop_enabled: true,
            goto_enabled: true,
            orbit_enabled: true,
            lock_enabled: true,
            rcs_enabled: true,
            point_defense_enabled: true,
            dock_enabled: true,
        }
    }
}

impl ShipCapabilities {
    /// Whether every capability is enabled - the authored default, which serde
    /// omits from content entirely.
    pub fn is_all_enabled(&self) -> bool {
        *self == Self::default()
    }
}

/// Serde's default for one capability field: a field nobody authored is on.
#[cfg(feature = "serde")]
fn enabled() -> bool {
    true
}

/// `skip_serializing_if` for one capability field, so only the WITHHELD ones
/// are written.
#[cfg(feature = "serde")]
fn is_enabled(enabled: &bool) -> bool {
    *enabled
}

/// The capabilities of every ship root, for the systems that gate on them.
///
/// Read through [`ship_capabilities`] rather than directly: a root with no
/// component at all is the all-enabled default, and every consumer has to
/// agree about that.
pub(crate) type ShipCapabilityQuery<'w, 's> = Query<'w, 's, &'static ShipCapabilities>;

/// What `ship` is permitted to do. A root carrying no [`ShipCapabilities`] -
/// a bare rig an example spawned - can do everything.
pub(crate) fn ship_capabilities(
    ship: Entity,
    capabilities: &ShipCapabilityQuery,
) -> ShipCapabilities {
    capabilities.get(ship).copied().unwrap_or_default()
}

/// Every LIVE flight computer in the world: the rows the attitude loop and the
/// rotation-authority check both read.
///
/// A live flight computer is a controller section that still has its
/// [`PDController`] (preview controllers have none) and is not disabled. The
/// PD is required through the DATA rather than a `With` filter because the
/// autopilot reads these same rows for the hull's rotation authority, so one
/// query answers both and the two can never disagree about which computers
/// count.
pub(crate) type LiveFlightComputers<'w, 's> = Query<
    'w,
    's,
    (&'static PDController, &'static ChildOf),
    (
        With<ControllerSectionMarker>,
        Without<SectionInactiveMarker>,
    ),
>;

/// Whether `ship` can still point itself: at least one live flight computer.
///
/// The PHYSICAL half of the old verb gate, and the only half a controller
/// answers. A ship that fails this cannot execute a commanded rotation, so
/// attitude-dependent maneuvers disengage - but its configured capabilities
/// are untouched.
pub(crate) fn ship_has_attitude_authority(ship: Entity, computers: &LiveFlightComputers) -> bool {
    computers.iter().any(|(_, &ChildOf(parent))| parent == ship)
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// The hull the gate is asked about.
    #[derive(Resource)]
    struct Subject(Entity);

    /// What the two gates answered for the subject.
    #[derive(Resource, Default)]
    struct Answers {
        capabilities: ShipCapabilities,
        attitude: bool,
    }

    fn collect_answers(
        subject: Res<Subject>,
        capabilities: ShipCapabilityQuery,
        computers: LiveFlightComputers,
        mut answers: ResMut<Answers>,
    ) {
        answers.capabilities = ship_capabilities(subject.0, &capabilities);
        answers.attitude = ship_has_attitude_authority(subject.0, &computers);
    }

    /// A live flight computer, as `controller_section` builds one: the marker
    /// plus the [`PDController`] that makes it a computer.
    fn live_computer(world: &mut World, ship: Entity) -> Entity {
        world
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_angular_acceleration: 40.0,
                    sustained_angular_speed: f32::INFINITY,
                },
            ))
            .id()
    }

    /// The editor's view ship, as `preview_controller_section` builds one: a
    /// [`ControllerSectionMarker`] with NO [`PDController`].
    fn preview_computer(world: &mut World, ship: Entity) -> Entity {
        world.spawn((ChildOf(ship), ControllerSectionMarker)).id()
    }

    fn answers(world: &mut World, ship: Entity) -> Answers {
        world.insert_resource(Subject(ship));
        world.init_resource::<Answers>();
        world.run_system_once(collect_answers).unwrap();
        let answers = world.resource::<Answers>();
        Answers {
            capabilities: answers.capabilities,
            attitude: answers.attitude,
        }
    }

    /// A preview controller carries no PD, so it is not a live flight computer
    /// and gives the hull no attitude authority.
    #[test]
    fn a_preview_controller_gives_no_attitude_authority() {
        let mut world = World::new();
        let ship = world.spawn_empty().id();
        preview_computer(&mut world, ship);

        assert!(!answers(&mut world, ship).attitude);
    }

    /// Delivery guard for the test above: a real computer does give it.
    #[test]
    fn a_live_computer_gives_attitude_authority() {
        let mut world = World::new();
        let ship = world.spawn_empty().id();
        live_computer(&mut world, ship);

        assert!(answers(&mut world, ship).attitude);
    }

    /// The property the section-owned model could not hold: losing every
    /// controller takes the ship's attitude authority and NOTHING else. LOCK,
    /// RCS and point defence are software decisions on the root, and a hulk
    /// still knows what it was configured with.
    #[test]
    fn losing_every_controller_leaves_the_configured_capabilities_alone() {
        let mut world = World::new();
        let configured = ShipCapabilities {
            goto_enabled: false,
            ..default()
        };
        let ship = world.spawn(configured).id();
        let computer = live_computer(&mut world, ship);

        let before = answers(&mut world, ship);
        assert!(before.attitude);
        assert_eq!(before.capabilities, configured);

        world.entity_mut(computer).despawn();

        let after = answers(&mut world, ship);
        assert!(
            !after.attitude,
            "the last controller is gone, so nothing executes a commanded rotation"
        );
        assert_eq!(
            after.capabilities, configured,
            "capabilities are the ROOT's, and no section death edits them"
        );
    }

    /// Two controllers stack attitude authority and never union capabilities:
    /// there is only one capability value on the hull to read.
    #[test]
    fn a_second_controller_adds_attitude_and_no_capability() {
        let mut world = World::new();
        let configured = ShipCapabilities {
            lock_enabled: false,
            ..default()
        };
        let ship = world.spawn(configured).id();
        let first = live_computer(&mut world, ship);
        live_computer(&mut world, ship);

        assert_eq!(answers(&mut world, ship).capabilities, configured);

        world.entity_mut(first).despawn();

        let after = answers(&mut world, ship);
        assert!(after.attitude, "one computer left still points the hull");
        assert_eq!(
            after.capabilities, configured,
            "a surviving controller cannot hand back a capability the root withheld"
        );
    }

    /// A bare rig - no capability component at all - can do everything, which
    /// is what keeps the examples and the section ranges flying and their
    /// point defence live.
    #[test]
    fn a_root_with_no_capability_component_can_do_everything() {
        let mut world = World::new();
        let ship = world.spawn_empty().id();

        assert!(answers(&mut world, ship).capabilities.is_all_enabled());
    }
}
