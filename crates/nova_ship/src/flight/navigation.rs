//! Leg navigation around sized bodies: the sphere-detour planner.
//!
//! `autopilot_step` plans every GOTO and GotoPos leg here, one tick at a time,
//! and flies the corner it gets back; the AI formation leg plans its own goal
//! the same way in the leader's frame.
//!
//! Nothing in this module decides policy. A caller states the clearance it
//! wants in a [`DetourPolicy`] and gets geometry back: there is no default
//! margin here, because what is safe for a skiff threading a belt is not what
//! is safe for a carrier.
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
    /// The mover's own radius (u), the part of [`Self::clearance`] that is
    /// metal. A body whose clearance already holds the mover waives the margin
    /// and blocks only a leg that would carry this much of hull into it.
    pub hull: f32,
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
/// A body whose clearance already contains `from` is judged on contact, not
/// on the margin: the mover is inside its bubble, so the margin is already
/// spent, and the leg counts as blocked only where it passes within
/// `body radius + hull` of the centre - the path that puts hull into rock. A
/// ship parked close to a rock (a mining park, a zero-standoff order) can
/// then fly away from it or around it, and never through it. The corner that
/// rounds such a body lies outside the bubble, so it steers the mover OUT.
///
/// A body whose clearance contains `to` is judged the same way: the leg's end
/// has already spent the margin, and a corner or a pushed goal that lands in
/// a neighbour's bubble is still a leg that must not cross that neighbour.
pub fn first_blocker(
    from: Vec3,
    to: Vec3,
    clearance: f32,
    hull: f32,
    obstacles: impl Iterator<Item = (Vec3, f32)>,
) -> Option<LegBlocker> {
    let leg = to - from;
    let len_sq = leg.length_squared();
    let mut best: Option<(f32, LegBlocker)> = None;
    for (center, radius) in obstacles {
        let bubble = radius + clearance;
        let bubble_sq = bubble * bubble;
        let endpoint_inside =
            from.distance_squared(center) < bubble_sq || to.distance_squared(center) < bubble_sq;
        let blocked_within = if endpoint_inside {
            radius + hull
        } else {
            bubble
        };
        let t = if len_sq > f32::EPSILON {
            ((center - from).dot(leg) / len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let closest = from + leg * t;
        if closest.distance_squared(center) >= blocked_within * blocked_within {
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
/// straight through them; flying AT the adjusted goal keeps the mover clear.
/// [`plan_leg`] pushes by the CLEAR clearance, so the adjusted goal sits
/// outside the band the held-corner check reads and the corner it took is
/// held, not dropped and re-taken every tick. Iterative because the pushed-out
/// point can land inside a neighbour's bubble; bounded so a pathological nest
/// cannot spin the loop.
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
    let goal = goal_outside(goal, policy.clear_clearance(), obstacles);
    route_leg(from, goal, held, policy, obstacles)
}

/// [`plan_leg`] to a goal that must not move: the goal is flown as given, not
/// pushed out of a bubble. A DOCK approach's corridor entry is fixed by the
/// port, and its selection already keeps it clear of every body.
pub(super) fn route_leg(
    from: Vec3,
    goal: Vec3,
    held: Option<Vec3>,
    policy: DetourPolicy,
    obstacles: &[(Vec3, f32)],
) -> LegPlan {
    let mut corner = held;
    if let Some(held) = corner {
        let clear = first_blocker(
            from,
            goal,
            policy.clear_clearance(),
            policy.hull,
            obstacles.iter().copied(),
        )
        .is_none();
        if clear || from.distance(held) <= policy.arrive_radius {
            corner = None;
        } else if let Some(blocker) = first_blocker(
            from,
            held,
            policy.clearance,
            policy.hull,
            obstacles.iter().copied(),
        ) {
            corner = Some(corner_around(from, held, blocker, policy));
        }
    }
    if corner.is_none() {
        corner = first_blocker(
            from,
            goal,
            policy.clearance,
            policy.hull,
            obstacles.iter().copied(),
        )
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
        hull: 4.0,
        hysteresis: 10.0,
        arrive_radius: 75.0,
    };
    const GOAL: Vec3 = Vec3::new(0.0, 0.0, -400.0);

    #[test]
    fn a_clear_leg_has_no_blocker() {
        let rocks = [(Vec3::new(0.0, 80.0, -200.0), 40.0)];
        assert!(first_blocker(
            Vec3::ZERO,
            GOAL,
            POLICY.clearance,
            POLICY.hull,
            rocks.iter().copied()
        )
        .is_none());
    }

    #[test]
    fn the_nearest_intruding_body_blocks_the_leg() {
        let near = (Vec3::new(10.0, 0.0, -150.0), 40.0);
        let far = (Vec3::new(-10.0, 0.0, -300.0), 40.0);
        let blocker = first_blocker(
            Vec3::ZERO,
            GOAL,
            POLICY.clearance,
            POLICY.hull,
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
            first_blocker(Vec3::ZERO, GOAL, 10.0, POLICY.hull, rocks.iter().copied()).is_none(),
            "a hull that wants 100 m of room threads this"
        );
        assert!(
            first_blocker(Vec3::ZERO, GOAL, 30.0, POLICY.hull, rocks.iter().copied()).is_some(),
            "a hull that wants 300 m does not"
        );
    }

    #[test]
    fn a_body_hugging_the_goal_blocks_only_where_the_hull_would_touch_it() {
        // The goal sits inside the rock's 20 u margin either way; the margin
        // is spent by the leg's end, so only hull contact (40 + 4 u) blocks.
        let hugging = [(GOAL + Vec3::new(0.0, 50.0, 0.0), 40.0)];
        assert!(
            first_blocker(
                Vec3::ZERO,
                GOAL,
                POLICY.clearance,
                POLICY.hull,
                hugging.iter().copied()
            )
            .is_none(),
            "a goal 50 u off the centre leaves the hull 6 u clear of the rock"
        );
        let swallowing = [(GOAL + Vec3::new(0.0, 30.0, 0.0), 40.0)];
        assert!(
            first_blocker(
                Vec3::ZERO,
                GOAL,
                POLICY.clearance,
                POLICY.hull,
                swallowing.iter().copied()
            )
            .is_some(),
            "a goal inside the rock itself is a leg into the rock"
        );
    }

    #[test]
    fn a_leg_ending_inside_a_bubble_is_still_blocked_through_the_body() {
        // Mover and leg end both inside the 40 + 20 u bubble of the rock at
        // 55 u: the leg through the centre is blocked, the leg that keeps the
        // hull off the rock (closest approach 49 u, past 40 + 4) flies.
        let rock = (Vec3::new(0.0, 0.0, -55.0), 40.0);
        let through = first_blocker(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -105.0),
            POLICY.clearance,
            POLICY.hull,
            [rock].iter().copied(),
        );
        assert!(
            through.is_some(),
            "a leg ending inside the bubble never flies through the rock"
        );
        let beside = first_blocker(
            Vec3::ZERO,
            Vec3::new(40.0, 0.0, -20.0),
            POLICY.clearance,
            POLICY.hull,
            [rock].iter().copied(),
        );
        assert!(
            beside.is_none(),
            "inside the bubble only hull contact blocks, got {beside:?}"
        );
    }

    #[test]
    fn a_goal_pushed_off_a_rock_face_keeps_its_corner_on_the_next_tick() {
        // The raw goal sits inside the rock; the planner pushes it out past
        // the clear band, so the body that pushed it is read by the clear
        // check and the corner taken on tick one is still held on tick two.
        let rock = (Vec3::new(0.0, 0.0, -200.0), 40.0);
        let raw_goal = Vec3::new(0.0, 0.0, -230.0);
        let first = plan_leg(Vec3::ZERO, raw_goal, None, POLICY, &[rock]);
        let corner = first
            .corner
            .expect("the leg to the far face crosses the rock");
        let pushed = goal_outside(raw_goal, POLICY.clear_clearance(), &[rock]);
        assert!(
            pushed.distance(rock.0) > rock.1 + POLICY.clear_clearance(),
            "the pushed goal sits outside the clear band"
        );
        let step = (corner - Vec3::ZERO).normalize() * 5.0;
        let second = plan_leg(step, raw_goal, Some(corner), POLICY, &[rock]);
        assert_eq!(
            second.corner,
            Some(corner),
            "the corner is held, not dropped and re-taken"
        );
    }

    #[test]
    fn a_body_whose_bubble_holds_the_mover_blocks_only_a_contact_leg() {
        // A ship parked 15 u off a 40 u rock, inside its 20 u margin. The leg
        // abeam passes 54 u from the centre - inside the margin, but with the
        // hull (4 u) clear of the rock - and flies; the leg through the centre
        // does not.
        let rock = (Vec3::new(0.0, 0.0, -55.0), 40.0);
        let clear_of_the_rock = Vec3::new(400.0, 0.0, -55.0);
        let blocked = first_blocker(
            Vec3::ZERO,
            GOAL,
            POLICY.clearance,
            POLICY.hull,
            [rock].iter().copied(),
        );
        assert!(
            blocked.is_some(),
            "a leg through the rock is blocked even from inside its bubble"
        );
        let grazing = first_blocker(
            Vec3::ZERO,
            clear_of_the_rock,
            POLICY.clearance,
            POLICY.hull,
            [rock].iter().copied(),
        );
        assert!(
            grazing.is_none(),
            "inside the bubble the margin is waived: a leg that carries no \
             hull into the rock flies, got {grazing:?}"
        );
    }

    #[test]
    fn a_mover_inside_a_bubble_is_steered_out_and_around() {
        let rock = (Vec3::new(0.0, 0.0, -55.0), 40.0);
        let plan = plan_leg(Vec3::ZERO, GOAL, None, POLICY, &[rock]);
        let corner = plan.corner.expect("the leg through the rock is blocked");
        assert!(corner.is_finite());
        assert!(
            corner.distance(rock.0) > rock.1 + POLICY.clearance,
            "the corner sits outside the bubble, so flying it leaves the rock"
        );
        // Flying the corner from the park never crosses the rock itself.
        assert!(
            first_blocker(Vec3::ZERO, corner, 0.0, POLICY.hull, [rock].iter().copied()).is_none(),
            "the corner leg keeps the hull off the rock"
        );
    }

    #[test]
    fn the_corner_clears_the_blocker_with_its_arrival_window() {
        let rocks = [(Vec3::new(15.0, 0.0, -200.0), 40.0)];
        let blocker = first_blocker(
            Vec3::ZERO,
            GOAL,
            POLICY.clearance,
            POLICY.hull,
            rocks.iter().copied(),
        )
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
        let blocker = first_blocker(
            Vec3::ZERO,
            GOAL,
            POLICY.clearance,
            POLICY.hull,
            rocks.iter().copied(),
        )
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
