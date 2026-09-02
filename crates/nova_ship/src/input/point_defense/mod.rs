//! Point defence, for whoever is flying: the per-turret torpedo
//! [`assignment`] both controllers share, and the [`ownership`] precedence that
//! decides which of the PLAYER's mounts the Flight Computer may borrow.
//!
//! The split is the whole design. Assignment answers "what should this mount
//! shoot", and it never cared who was aboard - it reads torpedoes, arcs and
//! time-to-impact and nothing else. Ownership answers "may the computer touch
//! this mount", which is a question only a player hull has, because an AI hull's
//! battery is the AI's outright.
//!
//! Touch this module to change what an unattended battery does. The gesture
//! side of the same story - what the raise and the lock MEAN - lives in
//! [`targeting`](super::targeting), and the capability that switches the whole
//! thing off is [`FlightVerb::PointDefense`] on the ship's computer.

mod assignment;
mod ownership;

use bevy::prelude::*;

// Reachable from the AI gun tests, which drive the real assignment pass rather
// than a stand-in for it.
pub(crate) use self::assignment::update_turret_point_defense;
use self::{
    assignment::insert_turret_defense_target,
    ownership::{
        draw_point_defense_lines, insert_point_defense_mount, point_defense_gizmo_config,
        update_point_defense_aim, update_point_defense_ownership, update_point_defense_trigger,
    },
};
pub use self::{
    assignment::TurretDefenseTarget,
    ownership::{
        flight_computer_works, MountAuthority, PointDefenseGizmos, PointDefenseMount,
        POINT_DEFENSE_REGRASP_SECS,
    },
};
use crate::{input::ai::AI_FIRE_RANGE_FACTOR, prelude::*};

/// The point-defence components, the ownership tiers and
/// `SpaceshipPointDefensePlugin`.
pub mod prelude {
    pub use super::{
        flight_computer_works, MountAuthority, PointDefenseGizmos, PointDefenseMount,
        SpaceshipPointDefensePlugin, SpaceshipPointDefenseSystems, TurretDefenseTarget,
        POINT_DEFENSE_REGRASP_SECS,
    };
}

/// The point-defence chain, as one ordering handle.
///
/// Named because two other chains have to sit AFTER it: the AI gun systems read
/// the assignment this set writes, and so does the player's own turret feed
/// (which skips the mounts the computer took).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipPointDefenseSystems;

/// Whether a mount may pull the trigger on `aim`.
///
/// TWO gates, and both are shared with the AI's own point-defence trigger
/// rather than restated: the RANGE gate (past the distance its bullets can
/// actually live, a shot is noise) and the 0.92 deg BEARING gate
/// ([`muzzle_on_target`], which is also the whole reachability rule - a mount
/// that cannot depress far enough never converges, so it never fires).
///
/// `anchor` is the target's live structure and `aim` the leaded point the
/// barrel actually steers to. The range is judged on the anchor and the bearing
/// on the lead, because a mount correctly leading a crossing target never bears
/// on the anchor itself and would otherwise hold fire forever.
pub(crate) fn mount_may_shoot(
    muzzle: &GlobalTransform,
    config: &TurretSectionConfig,
    anchor: Vec3,
    aim: Vec3,
) -> bool {
    let muzzle_position = muzzle.translation();
    let effective_range = config
        .muzzle_speed
        .over(config.projectile_lifetime)
        .to_engine()
        * AI_FIRE_RANGE_FACTOR;
    anchor.distance(muzzle_position) <= effective_range
        && muzzle_on_target(muzzle.forward().into(), muzzle_position, aim)
}

/// Runs point defence for both controllers: the shared per-turret assignment,
/// and the player-side ownership precedence, aim, trigger and gizmo. Added by
/// [`SpaceshipInputPlugin`](super::SpaceshipInputPlugin).
#[derive(Default)]
pub struct SpaceshipPointDefensePlugin {
    /// Whether the point-defence lines are drawn (false on headless runs, which
    /// have no camera to draw them for).
    pub render: bool,
}

impl Plugin for SpaceshipPointDefensePlugin {
    fn build(&self, app: &mut App) {
        trace!("SpaceshipPointDefensePlugin: build");

        app.register_type::<TurretDefenseTarget>();
        app.register_type::<PointDefenseMount>();
        app.register_type::<MountAuthority>();

        // The chain order IS the precedence. Ownership resolves first so
        // a mount the player just took is out of the pool the same tick;
        // assignment then works only what is left; the aim and trigger act on
        // what the assignment produced. Reading the ship's locks means the
        // whole chain also sits after the targeting systems that derive them.
        //
        // FIXED, not Update: the rounds this chain spends are spawned by
        // `shoot_spawn_projectile` on the fixed clock, and a decision taken on
        // the render clock is taken a variable number of times per unit of
        // SIMULATED time. One decision to one step is the invariant that keeps
        // a 144 Hz machine's point defence from outshooting a 30 Hz one.
        app.add_systems(
            FixedUpdate,
            (
                insert_turret_defense_target,
                insert_point_defense_mount,
                update_point_defense_ownership,
                update_turret_point_defense,
                update_point_defense_aim,
                update_point_defense_trigger,
            )
                .chain()
                .in_set(SpaceshipPointDefenseSystems)
                .after(super::targeting::SpaceshipTargetingSystems)
                .in_set(super::SpaceshipInputSystems),
        );

        if self.render {
            app.insert_gizmo_config(PointDefenseGizmos, point_defense_gizmo_config());
            // Reads muzzle and torpedo GlobalTransforms, so it runs after
            // transform propagation, like every other diegetic gizmo.
            app.add_systems(
                PostUpdate,
                draw_point_defense_lines.after(TransformSystems::Propagate),
            );
        }
    }
}
