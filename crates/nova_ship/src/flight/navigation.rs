//! Leg navigation around sized bodies: the sphere-detour planner.
//!
//! The autopilot flies BLIND straight legs - [`AutopilotAction::GotoPos`] has
//! no obstacle awareness of its own - so anything that wants a leg routed
//! around a rock plans the corners here and hands the autopilot one goal at a
//! time.
//!
//! Nothing in this module decides policy. A caller states the clearance it
//! wants in a [`DetourPolicy`] and gets geometry back: there is no default
//! margin here, because what is safe for a skiff threading a belt is not what
//! is safe for a carrier. The AI patrol is the only caller today.
//!
//! Engine units throughout: a world unit is 10 m.

use bevy::prelude::*;

pub mod prelude {
    pub use super::{plan_leg, DetourPolicy, LegPlan};
}

/// How much room a mover wants around the bodies on its leg.
///
/// Every field is the CALLER's, and all three are measured from the mover's
/// centre, because that is the point the flight computer flies: a hull's own
/// arm belongs in [`DetourPolicy::clearance`], not in the body's radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetourPolicy {
    /// Clearance (u) demanded outside a body's own radius before its leg
    /// counts as blocked: the mover's arm plus whatever margin it wants past
    /// its own skin.
    pub clearance: f32,
    /// Extra clearance (u) a DETOURING mover demands before it calls the
    /// direct leg clear again, so a leg grazing the margin cannot flip
    /// blocked/clear every tick - each flip moves the goal, and a moving goal
    /// resets the autopilot to its align phase for ever.
    pub hysteresis: f32,
    /// How near (u) the mover may be to a corner and call it reached.
    ///
    /// A corner is pushed out past this as well as past the clear check:
    /// anywhere the mover can call the corner reached must already see the
    /// direct leg comfortably clear, or the rounding stalls hopping corner to
    /// corner inside its own arrival window.
    pub arrive_radius: f32,
}

impl DetourPolicy {
    /// The clearance a leg must keep to be called CLEAR again, once the mover
    /// is already detouring.
    fn clear_clearance(&self) -> f32 {
        self.clearance + self.hysteresis
    }
}

/// A sized body found sitting on a leg.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegBlocker {
    /// The body's centre.
    pub center: Vec3,
    /// The body's own radius, with no clearance added.
    pub radius: f32,
    /// The point on the leg that passes nearest the centre.
    pub closest: Vec3,
}

/// What the planner says to fly this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegPlan {
    /// The position to fly at now.
    pub goal: Vec3,
    /// The corner to keep holding, or `None` when the direct leg is clear and
    /// the mover should fly at its real goal. A caller that stores the corner
    /// stores THIS, including when it is `None`.
    pub corner: Option<Vec3>,
}

/// The first sized body blocking the leg `from -> to`: its centre comes
/// within `body radius + clearance` of the leg, measured at the closest point
/// on the segment.
///
/// Two kinds of body are ignored. One whose clearance already contains `from`
/// - the mover is inside its bubble, this geometry cannot steer OUT of a
/// sphere, and calling it a blocker would spin corners around the mover's own
/// position. And one whose clearance contains `to`, because a fly-at-goal leg
/// is pushed outside every bubble by [`goal_outside`] before it is planned,
/// so this only guards degenerate geometry from looping.
pub fn first_blocker(
    from: Vec3,
    to: Vec3,
    clearance: f32,
    obstacles: impl Iterator<Item = (Vec3, f32)>,
) -> Option<LegBlocker> {
    let leg = to - from;
    let len_sq = leg.length_squared();
    let mut best: Option<(f32, LegBlocker)> = None;
    for (center, radius) in obstacles {
        let clearance = radius + clearance;
        let clearance_sq = clearance * clearance;
        if to.distance_squared(center) < clearance_sq
            || from.distance_squared(center) < clearance_sq
        {
            continue;
        }
        let t = if len_sq > f32::EPSILON {
            ((center - from).dot(leg) / len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let closest = from + leg * t;
        if closest.distance_squared(center) >= clearance_sq {
            continue;
        }
        if best.is_none_or(|(best_t, _)| t < best_t) {
            best = Some((
                t,
                LegBlocker {
                    center,
                    radius,
                    closest,
                },
            ));
        }
    }
    best.map(|(_, blocker)| blocker)
}

/// `goal`, pushed out of any sized body's clearance it sits inside - to the
/// nearest point on the bubble's surface, plus a step of slack.
///
/// A goal authored or scattered against a rock face is a routine hazard in a
/// dense band, and skipping such rocks in the blocker scan flies the leg
/// straight through them; flying AT the adjusted goal keeps the mover clear
/// while its own arrival check, which runs on the RAW goal, still turns the
/// route on time. Iterative because the pushed-out point can land inside a
/// neighbour's bubble; bounded so a pathological nest cannot spin the loop.
pub fn goal_outside(goal: Vec3, clearance: f32, obstacles: &[(Vec3, f32)]) -> Vec3 {
    let mut adjusted = goal;
    for _ in 0..4 {
        let Some((center, radius)) = obstacles.iter().copied().find(|(center, radius)| {
            let clearance = radius + clearance;
            adjusted.distance_squared(*center) < clearance * clearance
        }) else {
            return adjusted;
        };
        let out = (adjusted - center).try_normalize().unwrap_or(Vec3::Y);
        adjusted = center + out * (radius + clearance + 1.0);
    }
    adjusted
}

/// The corner that rounds `blocker`: pushed out from the body's centre
/// through the leg's closest point, past the clear check and past the
/// corner's own arrival window.
///
/// A body dead on the leg line has no side to prefer; any perpendicular
/// works, and the pick only has to be deterministic.
pub fn corner_around(from: Vec3, to: Vec3, blocker: LegBlocker, policy: DetourPolicy) -> Vec3 {
    let side = (blocker.closest - blocker.center)
        .try_normalize()
        .unwrap_or_else(|| {
            Dir3::new(to - from).map_or(Vec3::X, |leg| leg.any_orthonormal_vector())
        });
    blocker.center + side * (blocker.radius + policy.clear_clearance() + policy.arrive_radius)
}

/// Plan one tick of a leg from `from` to `goal`, given the corner the mover
/// is already holding.
///
/// A held corner is flown out until it is reached or the direct leg is
/// comfortably clear, and its OWN leg is re-validated every tick: momentum
/// and neighbours a single corner never saw can put a body on the way to it,
/// and a corner flown blind is exactly the crash the detour exists to
/// prevent. A blocked corner leg HOPS to a corner rounding that blocker. With
/// nothing held, a blocked leg takes a fresh corner and a clear leg flies
/// straight at the goal - pushed outside any bubble it was authored into.
///
/// A field is crossed one rounding at a time: reaching a corner with the leg
/// still blocked lands here again and picks the corner around the NEXT body.
pub fn plan_leg(
    from: Vec3,
    goal: Vec3,
    held: Option<Vec3>,
    policy: DetourPolicy,
    obstacles: &[(Vec3, f32)],
) -> LegPlan {
    let goal = goal_outside(goal, policy.clearance, obstacles);
    let mut corner = held;
    if let Some(held) = corner {
        let clear = first_blocker(
            from,
            goal,
            policy.clear_clearance(),
            obstacles.iter().copied(),
        )
        .is_none();
        if clear || from.distance(held) <= policy.arrive_radius {
            corner = None;
        } else if let Some(blocker) =
            first_blocker(from, held, policy.clearance, obstacles.iter().copied())
        {
            corner = Some(corner_around(from, held, blocker, policy));
        }
    }
    if corner.is_none() {
        corner = first_blocker(from, goal, policy.clearance, obstacles.iter().copied())
            .map(|blocker| corner_around(from, goal, blocker, policy));
    }
    LegPlan {
        goal: corner.unwrap_or(goal),
        corner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: DetourPolicy = DetourPolicy {
        clearance: 20.0,
        hysteresis: 10.0,
        arrive_radius: 75.0,
    };
    const GOAL: Vec3 = Vec3::new(0.0, 0.0, -400.0);

    #[test]
    fn a_clear_leg_has_no_blocker() {
        let rocks = [(Vec3::new(0.0, 80.0, -200.0), 40.0)];
        assert!(first_blocker(Vec3::ZERO, GOAL, POLICY.clearance, rocks.iter().copied()).is_none());
    }

    #[test]
    fn the_nearest_intruding_body_blocks_the_leg() {
        let near = (Vec3::new(10.0, 0.0, -150.0), 40.0);
        let far = (Vec3::new(-10.0, 0.0, -300.0), 40.0);
        let blocker = first_blocker(
            Vec3::ZERO,
            GOAL,
            POLICY.clearance,
            [far, near].iter().copied(),
        )
        .expect("both intrude; the leg is blocked");
        assert_eq!(
            (blocker.center, blocker.radius),
            near,
            "the FIRST body on the leg wins"
        );
    }

    #[test]
    fn a_wider_clearance_blocks_a_leg_a_narrow_one_flies() {
        // The whole point of the policy: the same rock and the same leg, and
        // the answer is the mover's, not the geometry's.
        let rocks = [(Vec3::new(55.0, 0.0, -200.0), 40.0)];
        assert!(
            first_blocker(Vec3::ZERO, GOAL, 10.0, rocks.iter().copied()).is_none(),
            "a hull that wants 100 m of room threads this"
        );
        assert!(
            first_blocker(Vec3::ZERO, GOAL, 30.0, rocks.iter().copied()).is_some(),
            "a hull that wants 300 m does not"
        );
    }

    #[test]
    fn a_body_hugging_the_goal_is_not_a_blocker() {
        let rocks = [(GOAL + Vec3::new(0.0, 30.0, 0.0), 40.0)];
        assert!(first_blocker(Vec3::ZERO, GOAL, POLICY.clearance, rocks.iter().copied()).is_none());
    }

    #[test]
    fn the_corner_clears_the_blocker_with_its_arrival_window() {
        let rocks = [(Vec3::new(15.0, 0.0, -200.0), 40.0)];
        let blocker = first_blocker(Vec3::ZERO, GOAL, POLICY.clearance, rocks.iter().copied())
            .expect("the rock sits on the leg");
        let corner = corner_around(Vec3::ZERO, GOAL, blocker, POLICY);
        assert!(
            corner.distance(rocks[0].0)
                > rocks[0].1 + POLICY.clearance + POLICY.hysteresis + POLICY.arrive_radius - 1.0,
            "the corner sits past the clear-check band plus its arrival window"
        );
    }

    #[test]
    fn a_dead_center_body_still_yields_a_corner() {
        let rocks = [(Vec3::new(0.0, 0.0, -200.0), 40.0)];
        let blocker = first_blocker(Vec3::ZERO, GOAL, POLICY.clearance, rocks.iter().copied())
            .expect("dead on the leg");
        let corner = corner_around(Vec3::ZERO, GOAL, blocker, POLICY);
        assert!(corner.is_finite());
        assert!(corner.distance(rocks[0].0) > rocks[0].1 + POLICY.clearance);
    }

    #[test]
    fn a_blocked_leg_is_planned_around_and_a_clear_one_is_not() {
        let rocks = [(Vec3::new(15.0, 0.0, -200.0), 40.0)];
        let blocked = plan_leg(Vec3::ZERO, GOAL, None, POLICY, &rocks);
        assert_eq!(
            blocked.corner,
            Some(blocked.goal),
            "a blocked leg flies its corner and holds it"
        );
        let clear = plan_leg(Vec3::ZERO, GOAL, None, POLICY, &[]);
        assert_eq!(
            clear,
            LegPlan {
                goal: GOAL,
                corner: None
            },
            "a clear leg flies the goal and holds nothing"
        );
    }

    #[test]
    fn a_held_corner_is_dropped_once_the_direct_leg_is_comfortably_clear() {
        // The rock is off the leg by more than the clear check wants, so a
        // mover still holding a corner for it is done detouring.
        let rocks = [(Vec3::new(90.0, 0.0, -200.0), 40.0)];
        let held = Vec3::new(120.0, 0.0, -200.0);
        let plan = plan_leg(Vec3::ZERO, GOAL, Some(held), POLICY, &rocks);
        assert_eq!(plan.corner, None, "the leg is clear; let the corner go");
        assert_eq!(plan.goal, GOAL);
    }

    #[test]
    fn a_corner_with_a_body_in_the_way_hops_to_another() {
        // A corner is a goal like any other: a body between the mover and it
        // is a body the mover would fly into.
        let on_the_leg = (Vec3::new(15.0, 0.0, -200.0), 40.0);
        let on_the_corner = (Vec3::new(40.0, 0.0, -100.0), 40.0);
        let held = Vec3::new(80.0, 0.0, -200.0);
        let plan = plan_leg(
            Vec3::ZERO,
            GOAL,
            Some(held),
            POLICY,
            &[on_the_leg, on_the_corner],
        );
        let corner = plan.corner.expect("the leg is still blocked");
        assert_ne!(corner, held, "the corner leg was blocked, so it hopped");
        assert!(
            corner.distance(on_the_corner.0) > on_the_corner.1 + POLICY.clearance,
            "and the hop clears what blocked it, got {corner:?}"
        );
    }

    #[test]
    fn a_goal_inside_a_bubble_is_flown_to_its_boundary() {
        let rocks = [(GOAL + Vec3::new(0.0, 30.0, 0.0), 40.0)];
        let plan = plan_leg(Vec3::ZERO, GOAL, None, POLICY, &rocks);
        assert!(
            plan.goal.distance(rocks[0].0) >= rocks[0].1 + POLICY.clearance,
            "the flown goal is pushed out of the bubble, got {:?}",
            plan.goal
        );
    }
}
