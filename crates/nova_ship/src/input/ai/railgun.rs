//! AI railgun shots: the commit envelope (bore alignment plus reach) and the
//! per-gun cadence on top of the lance's own reload.
//!
//! DELIBERATELY CRUDE, and recorded as such. The AI does not FLY to make the
//! shot - it commits when its orbit happens to sweep the bore across a target
//! it is already fighting. A raider therefore lands the occasional lance hit
//! and never sets one up, which is the honest half of "the AI may use it".
//! The lance run - break the orbit, roll onto the line, commit, peel off - is
//! task `20260901-104359`, and this module is what that task replaces.
//!
//! The alignment gate is TIGHT where the torpedo's is loose, and the reason is
//! the difference between the two weapons: a torpedo turns after launch, so a
//! rough bearing is enough, while a slug goes exactly where the hull pointed
//! when the charge finished. A loose gate here would mean a raider spending
//! its one shell down an empty line every reload.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{guns::ai_line_of_fire_blocked, maneuver::ai_target_anchor};
use crate::prelude::*;

/// Per-gun commit cadence (s) on top of the lance's own reload. Keeps an AI
/// lance to deliberate, spaced shots even when a long orbit holds the bore on
/// a target: the reload is the gun's floor, this is the pilot's. Playtest
/// knob.
const AI_RAILGUN_COOLDOWN_SECS: f32 = 14.0;

/// Bore-alignment gate (cos) on the COMMIT. About 8 degrees.
///
/// Tight, and it has to be: the shot lands one charge time later along
/// whatever line the hull holds then, so this is the AI betting that its
/// current heading survives the charge. Anything looser is a wasted shell.
/// Playtest knob.
const AI_RAILGUN_ALIGNMENT_COS: f32 = 0.99;

/// Fraction of the slug's own reach the AI will commit inside.
///
/// The slug expires on `slug_lifetime`, so `slug_speed * slug_lifetime` is
/// the hard reach; committing well inside it leaves room for the target to
/// open the range during the charge and the flight.
const AI_RAILGUN_REACH_FACTOR: f32 = 0.6;

/// Per-gun AI commit state. Lazily inserted by
/// [`update_railgun_section_input`] on lances whose ship is AI-controlled -
/// an Add-observer would race the root's `AISpaceshipMarker`, which lands
/// after the child sections spawn, exactly as for [`AITorpedoBay`].
///
/// [`AITorpedoBay`]: super::AITorpedoBay
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIRailgun {
    /// Time until this gun may commit again. A fresh [`Cooldown`] is READY, so
    /// the first shot of a fight comes as soon as the envelope opens.
    cooldown: Cooldown,
}

impl Default for AIRailgun {
    fn default() -> Self {
        Self {
            cooldown: Cooldown::new(AI_RAILGUN_COOLDOWN_SECS),
        }
    }
}

/// The geometric half of the commit: the target inside
/// [`AI_RAILGUN_REACH_FACTOR`] of the slug's reach, with the BORE - not the
/// hull's nose, not the anchor bearing - on it. Pure, for unit testing.
fn ai_railgun_envelope(to_target: Vec3, bore: Vec3, reach: f32) -> bool {
    let distance = to_target.length();
    if distance <= f32::EPSILON || distance > reach * AI_RAILGUN_REACH_FACTOR {
        return false;
    }
    bore.dot(to_target / distance) > AI_RAILGUN_ALIGNMENT_COS
}

/// Pull each AI ship's railgun triggers: write [`RailgunSectionInput`] on its
/// lances when the commit envelope is open.
///
/// The per-ship gates match the bay's - an Engage-like state (Evade excluded:
/// a jinking hull cannot hold a bore through a charge) and a SHIP target, since
/// a torpedo is the guns' problem and not worth a shell. Per ship, the line of
/// fire must be clear, so no lance is spent on the cover in front of it. Per
/// gun: the cadence elapsed and the envelope open.
///
/// The trigger is HELD while the envelope stays open, exactly as the bay's is,
/// and the cadence is burned by [`on_railgun_fired_burn_ai_cadence`] when a
/// shell actually leaves. This system runs in `Update` while
/// `charge_and_fire_railgun` consumes the trigger in `FixedUpdate`, so a
/// one-frame pulse would be missed entirely on any frame that runs no fixed
/// step - above the 64 Hz fixed rate that is most frames, and each miss used
/// to cost a full cadence for a shot that never happened.
#[expect(
    clippy::type_complexity,
    reason = "one query per railgun lifecycle stage"
)]
pub(super) fn update_railgun_section_input(
    time: Res<Time>,
    mut commands: Commands,
    q_missing: Query<(Entity, &ChildOf), (With<RailgunSectionMarker>, Without<AIRailgun>)>,
    mut q_section: Query<
        (
            &mut RailgunSectionInput,
            &mut AIRailgun,
            &RailgunEngineFigures,
            &RailgunCharge,
            Option<&SectionAmmo>,
            &GlobalTransform,
            &ChildOf,
        ),
        (
            With<RailgunSectionMarker>,
            // A disabled lance cannot fire, so it must not be pulled on either.
            Without<SectionInactiveMarker>,
        ),
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    spatial: SpatialQuery,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
) {
    for (section, ChildOf(parent)) in &q_missing {
        if q_spaceship.contains(*parent) {
            commands.entity(section).insert(AIRailgun::default());
        }
    }

    let dt = time.delta_secs();
    for (mut input, mut gun, figures, charge, ammo, section_pose, &ChildOf(spaceship)) in
        &mut q_section
    {
        gun.cooldown.tick(dt);
        let commit = commit_is_open(
            &gun,
            figures,
            charge,
            ammo,
            section_pose,
            spaceship,
            &q_spaceship,
            &q_target,
            &q_ship_root,
            &spatial,
            &q_sensor,
            &q_collider_of,
        );
        // HELD, not pulsed, and released explicitly the moment any gate shuts -
        // the bay's rule, for the bay's reason. `set_if_neq` by hand, so a
        // steady decision does not dirty the component every frame.
        if **input != commit {
            **input = commit;
        }
    }
}

/// Whether this lance's commit envelope is open THIS frame. Split out so the
/// gates read as one decision rather than as an early-return chain that has to
/// leave the trigger in the right state on every exit.
#[expect(
    clippy::too_many_arguments,
    reason = "the gates the commit reads, passed rather than re-queried"
)]
fn commit_is_open(
    gun: &AIRailgun,
    figures: &RailgunEngineFigures,
    charge: &RailgunCharge,
    ammo: Option<&SectionAmmo>,
    section_pose: &GlobalTransform,
    spaceship: Entity,
    q_spaceship: &Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: &Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_ship_root: &Query<(), With<SpaceshipRootMarker>>,
    spatial: &SpatialQuery,
    q_sensor: &Query<(), With<Sensor>>,
    q_collider_of: &Query<&ColliderOf>,
) -> bool {
    let Ok((ship, transform, com, state, target)) = q_spaceship.get(spaceship) else {
        return false;
    };
    if !gun.cooldown.ready() {
        return false;
    }
    // Already committed: nothing to decide until the shell is away.
    if *charge != RailgunCharge::Ready {
        return false;
    }
    // An empty magazine cannot commit, so pulling on one would burn the cadence
    // on a shot the section refuses to take.
    if ammo.is_some_and(SectionAmmo::is_empty) {
        return false;
    }
    if !state.engages() || *state == AIBehaviorState::Evade {
        return false;
    }
    let Some(target_ship) = (**target).filter(|&target| q_ship_root.contains(target)) else {
        return false;
    };
    let Some(target_anchor) = ai_target_anchor(Some(target_ship), q_target) else {
        return false;
    };

    // The BORE, read off the section's own pose. A lance cannot traverse,
    // so this is the ship's heading through the mount it was bolted on
    // with - and a lance bolted on sideways points sideways, which is the
    // builder's problem and not something the AI corrects for.
    let bore = section_pose.rotation() * Vec3::NEG_Z;
    let own_anchor = live_structure_anchor(transform, com);
    if !ai_railgun_envelope(target_anchor - own_anchor, bore, figures.reach) {
        return false;
    }
    !ai_line_of_fire_blocked(
        spatial,
        q_sensor,
        q_collider_of,
        ship,
        target_ship,
        own_anchor,
        target_anchor,
    )
}

/// Burn the AI cadence when a shell actually LEAVES, not when the commit is
/// decided: a lance ignored - because the frame ran no fixed step, or because
/// the section refused - never spends its pilot's cooldown. The bay's rule
/// (`a bay ignored never burns the cooldown`), for the bay's reason.
pub(super) fn on_railgun_fired_burn_ai_cadence(
    fired: On<RailgunFired>,
    mut q_gun: Query<&mut AIRailgun>,
) {
    if let Ok(mut gun) = q_gun.get_mut(fired.entity) {
        gun.cooldown.trigger();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate is about the BORE and the reach, and it is tight on purpose:
    /// a shot lands a charge later along the line the hull holds then.
    #[test]
    fn the_commit_envelope_wants_the_bore_on_the_target_and_the_target_in_reach() {
        let reach = 1_000.0;
        let ahead = Vec3::NEG_Z * 400.0;

        assert!(
            ai_railgun_envelope(ahead, Vec3::NEG_Z, reach),
            "bore on the target, well inside reach"
        );
        assert!(
            !ai_railgun_envelope(Vec3::NEG_Z * 900.0, Vec3::NEG_Z, reach),
            "past the commit fraction of the slug's reach"
        );
        // ~11 degrees off: further than the gate allows, and at 400u that is
        // ~78u of miss - most of a hull's length past the target.
        let off_axis = Quat::from_rotation_y(0.2) * Vec3::NEG_Z;
        assert!(
            !ai_railgun_envelope(ahead, off_axis, reach),
            "the bore is off the target's line"
        );
        assert!(
            !ai_railgun_envelope(Vec3::ZERO, Vec3::NEG_Z, reach),
            "a degenerate bearing is not a shot"
        );
    }
}
