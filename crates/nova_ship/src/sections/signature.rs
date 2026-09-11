//! What a hull looks like to a radar, derived from the hull.
//!
//! A ship's [`LockSignature`] is published here every tick from its LIVE
//! sections, the way [`HullRadius`](super::hull_radius::HullRadius) is: a hull
//! returns on its metal and on the machinery it is running. Lose sections and
//! it shrinks; lose the last flight computer and the hull stops SEEING as well
//! ([`SensorsDark`]), because the scanner is the computer's.
//!
//! The target's half of the range model lives here; the observer's half is
//! [`SensorRange`](crate::prelude::SensorRange), and the pass that puts them
//! together is `input/targeting/sensing.rs`.
//!
//! METERS, then the seam: the formula is authored in meters because every
//! number in it is a size a designer reasons about, and the published
//! component is world units because the sensor pass compares it against an
//! avian position every frame.

use bevy::{ecs::entity::EntityHashMap, prelude::*};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::{
    ControllerSectionMarker, RailgunSectionMarker, SectionInactiveMarker, SectionMarker,
    SpaceshipRootMarker, TorpedoSectionMarker, TurretSectionMarker,
};

use super::{
    controller_section::{prelude::ControllerSectionTuning, DEFAULT_MAX_TORQUE},
    hull_radius::prelude::HullRadius,
    thruster_section::{prelude::ThrusterSectionMagnitude, THRUSTER_SECTION_DEFAULT_MAGNITUDE},
};
use crate::prelude::LockSignature;

/// The `SensorsDark` marker and the signature terms.
pub mod prelude {
    pub use super::{ship_lock_signature, SensorsDark, SIGNATURE_BASE, SIGNATURE_PER_ARM};
}

/// What a hull returns before anything is installed on it, in meters.
///
/// The floor of the model: a bare one-cell hull is a metal box in a vacuum,
/// and a metal box is not invisible. With the arm term it puts a single cell
/// at about 9 km of lock range, close enough to matter to a picket and far
/// short of a warship.
pub const SIGNATURE_BASE: Meters = Meters(280.0);

/// Signature per meter of structural arm.
///
/// A ratio, not a length: the arm is how much hull there is to bounce a wave
/// off, and it is the one term that cannot be switched off. It dominates on a
/// capital and is nearly silent on a needle.
pub const SIGNATURE_PER_ARM: f32 = 5.5;

/// Signature per installed SYSTEM, in meters, before the logarithm.
///
/// Drives, computers and weapons are each worth this much at one standard
/// unit, and each stack is compressed by `ln(1 + units)`: the second drive
/// adds far less than the first, and the fortieth adds almost nothing. A
/// capital is loud because it is big, not because it is full.
const SIGNATURE_PER_SYSTEM: Meters = Meters(100.0);

/// Present on a ship root that cannot SEE: it carries flight computers and not
/// one of them is live, so the hull has no scanner however much of it is still
/// there.
///
/// HAD one and lost it, the same test `integrity/neutralize.rs` applies to the
/// guns: a hull built without a computer at all is not a hulk, it is a hull
/// whose sensors are somebody else's business, and a scenario that wants it
/// blind authors `sensor_range: Some(0.0)`.
///
/// A hulk is still a target - it keeps a [`LockSignature`] - and simply stops
/// being an observer. Derived every tick beside the signature, never
/// authored.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct SensorsDark;

/// One hull's radar signature, in meters, from what is live on it.
///
/// `SIGNATURE_BASE + SIGNATURE_PER_ARM * arm + SIGNATURE_PER_SYSTEM *
/// (ln(1 + drives) + ln(1 + computers) + ln(1 + weapons))`, where a drive unit
/// is one standard thruster's thrust and a computer unit is one standard
/// flight computer's torque, so a capital drive counts for the many basic
/// ones it replaces rather than for one section.
///
/// Throttle and trigger do not appear: this is what the hull IS, not what it
/// is doing. A visible-by-burning term is a stealth mechanic and belongs to
/// its own transient number when there is one.
pub fn ship_lock_signature(
    arm: Meters,
    drive_units: f32,
    controller_units: f32,
    live_weapons: f32,
) -> Meters {
    let stack = |units: f32| (1.0 + units.max(0.0)).ln();
    Meters(
        SIGNATURE_BASE.get()
            + SIGNATURE_PER_ARM * arm.get().max(0.0)
            + SIGNATURE_PER_SYSTEM.get()
                * (stack(drive_units) + stack(controller_units) + stack(live_weapons)),
    )
}

/// What one pass counts on one hull.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Installed {
    /// Summed thrust over the live drives, in standard thruster units.
    drives: f32,
    /// Summed torque over the live flight computers, in standard units.
    controllers: f32,
    /// Live weapon sections: turrets, bays and railguns together.
    weapons: f32,
    /// Whether any flight computer is still live.
    lit: bool,
    /// Whether the hull carries a flight computer at all, live or dead.
    helm: bool,
}

/// Publish every ship's [`LockSignature`] and its [`SensorsDark`] state from
/// its live sections.
///
/// ONE pass over every live section rather than one per hull, for the reason
/// `publish_hull_radii` gives: re-filtering the section query per hull is
/// quadratic in a busy scene.
///
/// A root with nothing live left on it answers with the base alone and goes
/// DARK: a beaten hull is still metal, so it is still a contact, and there is
/// nothing running aboard it either to be loud with or to see with. A root
/// avian has not measured yet has no arm, which reads as a point and is the
/// honest answer while its colliders are still being linked.
pub(crate) fn publish_ship_signatures(
    // Reused across ticks: this runs for every hull on every fixed tick and
    // must not allocate per ship per tick.
    mut installed: Local<EntityHashMap<Installed>>,
    mut commands: Commands,
    mut q_root: Query<
        (
            Entity,
            Option<&HullRadius>,
            Option<&mut LockSignature>,
            Has<SensorsDark>,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_section: Query<
        (
            &ChildOf,
            Has<SectionInactiveMarker>,
            Option<&ThrusterSectionMagnitude>,
            Option<&ControllerSectionTuning>,
            Has<ControllerSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Has<RailgunSectionMarker>,
        ),
        With<SectionMarker>,
    >,
) {
    installed.clear();
    for (&ChildOf(root), inactive, thrust, tuning, is_controller, turret, bay, railgun) in
        &q_section
    {
        if q_root.get(root).is_err() {
            continue;
        }
        let entry = installed.entry(root).or_default();
        entry.helm |= is_controller;
        if inactive {
            continue;
        }
        if let Some(thrust) = thrust {
            entry.drives += (**thrust).max(0.0) / THRUSTER_SECTION_DEFAULT_MAGNITUDE;
        }
        if let Some(tuning) = tuning {
            entry.controllers += tuning.max_torque.max(0.0) / DEFAULT_MAX_TORQUE;
        }
        entry.lit |= is_controller;
        if turret || bay || railgun {
            entry.weapons += 1.0;
        }
    }

    for (root, arm, signature, dark) in &mut q_root {
        let live = installed.get(&root).copied().unwrap_or_default();
        // Engine boundary: the arm is measured off avian colliders, so it
        // arrives in world units and the formula wants meters.
        let published = LockSignature(
            ship_lock_signature(
                Meters::from_engine(arm.map_or(0.0, |arm| **arm)),
                live.drives,
                live.controllers,
                live.weapons,
            )
            .to_engine(),
        );
        match signature {
            Some(mut signature) => {
                signature.set_if_neq(published);
            }
            None => {
                commands.entity(root).try_insert(published);
            }
        }
        match (live.helm && !live.lit, dark) {
            (true, false) => {
                commands.entity(root).try_insert(SensorsDark);
            }
            (false, true) => {
                commands.entity(root).try_remove::<SensorsDark>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// A hull with nothing installed and no arm at all: the model's floor.
    #[test]
    fn a_bare_hull_is_not_invisible() {
        let bare = ship_lock_signature(Meters::ZERO, 0.0, 0.0, 0.0);
        assert_eq!(
            bare, SIGNATURE_BASE,
            "an unequipped hull returns the base and nothing else"
        );
        assert!(
            bare > Meters::ZERO,
            "and a metal box in a vacuum is never invisible"
        );
    }

    /// The term that cannot be switched off: a bigger hull is a louder hull,
    /// whatever is bolted to it.
    #[test]
    fn a_longer_arm_returns_more() {
        let skiff = ship_lock_signature(Meters(47.8), 2.0, 1.0, 0.0);
        let carrier = ship_lock_signature(Meters(194.2), 4.0, 10.0, 0.0);
        assert!(
            carrier > skiff,
            "the carrier has to answer louder than the skiff: {carrier:?} against {skiff:?}"
        );
    }

    /// Why the stacks are logarithms: a capital carries dozens of each, and a
    /// linear term would make it visible from the far side of the map.
    #[test]
    fn each_extra_system_adds_less_than_the_one_before() {
        let at = |units: f32| ship_lock_signature(Meters::ZERO, units, 0.0, 0.0);
        let first = at(1.0) - at(0.0);
        let tenth = at(10.0) - at(9.0);
        assert!(
            first > tenth * 5.0,
            "the first drive has to be worth far more than the tenth: {first:?} against {tenth:?}"
        );
        assert!(
            tenth > Meters::ZERO,
            "but the tenth still counts for something"
        );
    }

    /// A hull answers for what is LIVE on it, and stops seeing when the last
    /// computer dies.
    #[test]
    fn the_pass_publishes_a_hull_and_darkens_a_hulk() {
        let mut world = World::new();
        let ship = world
            .spawn((SpaceshipRootMarker, HullRadius(Meters(50.0).to_engine())))
            .id();
        let computer = world
            .spawn((
                ChildOf(ship),
                SectionMarker,
                ControllerSectionMarker,
                ControllerSectionTuning {
                    steering_lag: 0.5,
                    max_torque: DEFAULT_MAX_TORQUE,
                },
            ))
            .id();
        world.spawn((
            ChildOf(ship),
            SectionMarker,
            ThrusterSectionMagnitude(THRUSTER_SECTION_DEFAULT_MAGNITUDE),
        ));

        world.run_system_once(publish_ship_signatures).unwrap();

        let whole = world
            .get::<LockSignature>(ship)
            .copied()
            .expect("the pass publishes a signature");
        assert_eq!(
            Meters::from_engine(*whole),
            ship_lock_signature(Meters(50.0), 1.0, 1.0, 0.0),
            "the published signature is the model read off the live sections"
        );
        assert!(
            !world.entity(ship).contains::<SensorsDark>(),
            "a hull with a working computer can see"
        );

        world.entity_mut(computer).insert(SectionInactiveMarker);
        world.run_system_once(publish_ship_signatures).unwrap();

        assert!(
            world.entity(ship).contains::<SensorsDark>(),
            "losing the last flight computer blinds the hull"
        );
        let beaten = world
            .get::<LockSignature>(ship)
            .copied()
            .expect("a hulk is still a target");
        assert!(
            *beaten < *whole,
            "and it answers quieter than it did whole: {beaten:?} against {whole:?}"
        );
    }
}
