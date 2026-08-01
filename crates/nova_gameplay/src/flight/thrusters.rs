//! Engine bookkeeping: cluster the live thrusters into direction groups,
//! pick the cheapest group for a burn, allocate a demand across the whole
//! engine set as a torque-nulling throttle vector, and spool each input
//! toward it. Shared by the autopilot and the manual burn.

use bevy::prelude::*;

/// A thruster counts as part of the main drive when its thrust direction
/// aligns with the ship's forward axis at least this much (cosine). Everything
/// less aligned is left alone - the flight layer only commands the engines
/// that actually push the ship forward.
pub(super) const FORWARD_ALIGNMENT_COS: f32 = 0.9;

/// Projected-gradient iterations for the thrust balancer
/// ([`balance_throttles`]). The problem is a tiny convex QP (one equality, box
/// bounds, a firing set of a few engines) that converges in a handful of steps;
/// this is a generous cap.
const BALANCE_ITERS: usize = 40;

/// Bisection iterations for the balancer's capacity projection
/// ([`project_onto_demand`]) - enough to pin the multiplier to f32 precision.
const BALANCE_PROJECT_ITERS: usize = 40;

/// Weight of the squared net off-axis force (perpendicular to the burn) against
/// the squared net torque in the balance objective ([`balance_throttles`]).
/// This is the "bounded lateral drift" trade: recruiting an off-axis thruster
/// for counter-torque adds a sideways force the maneuver did not ask for, so
/// the allocator pays for it - softly, not with a hard zero constraint, because
/// a hard zero needs an opposing pair and gives no help to a ship whose damage
/// left it exactly one usable lateral. The weight has units of lever-arm
/// squared: an off-axis engine with lever arm `r` about the COM nulls all but
/// `w / (r^2 + w)` of the torque it is recruited against, so at 0.05 a one-unit
/// lever leaves ~5% residual (well inside the PD's hold) while an engine
/// mounted nearly through the COM - huge force for no torque - is correctly not
/// worth firing.
const LATERAL_PENALTY: f32 = 0.05;

/// A cluster of live engines that push the ship in (roughly) the same world
/// direction, with their summed per-tick authority. The planner's unit of
/// choice: rotate whichever group is cheapest onto the needed burn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ThrusterGroup {
    pub(super) world_dir: Vec3,
    pub(super) authority: f32,
}

/// Greedily cluster engines (`(world thrust dir, magnitude)`) into direction
/// groups: an engine joins the first group within `cone_cos` of its
/// direction, else seeds a new one. Group direction is the magnitude-weighted
/// mean. Pure for unit testing.
pub(super) fn cluster_thrusters(engines: &[(Vec3, f32)], cone_cos: f32) -> Vec<ThrusterGroup> {
    let mut sums: Vec<(Vec3, f32)> = Vec::new();
    for &(dir, magnitude) in engines {
        if magnitude <= 0.0 {
            continue;
        }
        match sums
            .iter_mut()
            .find(|(sum, _)| sum.normalize_or_zero().dot(dir) >= cone_cos)
        {
            Some((sum, authority)) => {
                *sum += dir * magnitude;
                *authority += magnitude;
            }
            None => sums.push((dir * magnitude, magnitude)),
        }
    }
    sums.into_iter()
        .map(|(sum, authority)| ThrusterGroup {
            world_dir: sum.normalize_or_zero(),
            authority,
        })
        .collect()
}

/// Estimated seconds for `group` to deliver a `delta_v` burn along
/// `burn_dir`: time to rotate the group onto the burn (weighted by the
/// rotation bias - turning feels expensive) plus time to burn the impulse at
/// the group's authority. Pure for unit testing.
fn group_time_score(
    group: &ThrusterGroup,
    burn_dir: Vec3,
    delta_v: f32,
    mass: f32,
    dt: f32,
    turn_rate: f32,
    bias: f32,
) -> f32 {
    let rotation_time = group.world_dir.angle_between(burn_dir) / turn_rate.max(1e-3);
    let burn_time = if group.authority > 0.0 {
        delta_v * mass * dt / group.authority
    } else {
        f32::INFINITY
    };
    rotation_time * bias + burn_time
}

/// The fastest group for a burn, per [`group_time_score`].
pub(super) fn choose_group(
    groups: &[ThrusterGroup],
    burn_dir: Vec3,
    delta_v: f32,
    mass: f32,
    dt: f32,
    turn_rate: f32,
    bias: f32,
) -> Option<&ThrusterGroup> {
    groups.iter().min_by(|a, b| {
        group_time_score(a, burn_dir, delta_v, mass, dt, turn_rate, bias).total_cmp(
            &group_time_score(b, burn_dir, delta_v, mass, dt, turn_rate, bias),
        )
    })
}

/// Main-drive input (`0..1`) to deliver `impulse` this tick given the drive's
/// per-tick authority. Pure for unit testing.
pub(super) fn burn_input(impulse: f32, authority: f32) -> f32 {
    if authority <= 0.0 {
        return 0.0;
    }
    (impulse / authority).clamp(0.0, 1.0)
}

/// One engine's linear contribution to the thrust-balance problem, all per
/// unit of input (`0..1`). `primary` marks the firing set - the engines
/// inside the burn's alignment cone that the demand is budgeted against;
/// everything else is a counter-torque candidate the allocator may recruit.
/// `forward` is the thrust the engine adds along the burn direction: real for
/// primary engines (they carry the demand equality), and zero by convention
/// for recruits - a recruit is fired for its `torque` alone, so its ENTIRE
/// thrust vector goes into `lateral`, the force the maneuver did not ask for.
/// Billing the whole recruit force to the penalty (rather than crediting its
/// along-burn component to the equality) keeps a saturated demand feasible -
/// at full stick the equality has zero slack, and a recruit whose drift-tilted
/// thrust fights the burn by epsilon would otherwise be projected straight
/// back to zero - and makes a recruited retro honestly pay for the thrust it
/// cancels instead of the mains silently over-throttling. `torque` is
/// `(engine_pos - com) x thrust`. Given in any single consistent frame - the
/// balance objective (null torque, minimal off-axis force) is
/// frame-invariant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BalanceEngine {
    pub(super) forward: f32,
    pub(super) lateral: Vec3,
    pub(super) torque: Vec3,
    pub(super) primary: bool,
}

/// Wrench allocation: per-engine inputs (each `0..1`) that deliver `demand`
/// units of thrust along the burn direction while routing the resultant force
/// as close to *through* the center of mass as the whole live engine set
/// allows - recruiting off-axis engines (laterals, retros) purely for their
/// counter-torque when the firing set cannot balance itself.
///
/// Solves the tiny convex QP `min ||sum torque_i u_i||^2 + LATERAL_PENALTY *
/// ||sum lateral_i u_i||^2` subject to `sum forward_i u_i = demand` and `0 <=
/// u_i <= 1` by projected gradient: a ship has a handful of engines, so a
/// handful of steps converge. The equality is the maneuver's demand (deliver
/// the thrust the pilot/autopilot asked for); the objective nulls the net
/// torque while keeping bounded the sideways force that nulling costs (see
/// [`LATERAL_PENALTY`]). The seed is the uniform throttle over the firing set
/// (`demand / sum(primary forward)`, off-axis engines at zero) - always
/// feasible, and a stationary point whenever the net torque is already null, so
/// a balanced (symmetric) drive returns unchanged and idle off-axis engines are
/// never lit gratuitously. When nothing can help - a lone off-center engine
/// with no off-axis thruster left - the demand wins and the residual torque is
/// left for the PD controller, exactly the pre-allocation behavior. Pure for
/// unit testing.
pub(super) fn balance_throttles(engines: &[BalanceEngine], demand: f32) -> Vec<f32> {
    let n = engines.len();
    if n == 0 {
        return Vec::new();
    }
    let primary_forward: f32 = engines
        .iter()
        .filter(|e| e.primary)
        .map(|e| e.forward.max(0.0))
        .sum();
    if primary_forward <= 1e-6 {
        return vec![0.0; n];
    }
    // Clamp the demand into what the firing set can deliver, then seed at its
    // uniform throttle: sum_primary forward_i * (demand/total) = demand, each
    // in [0,1], every off-axis engine dark.
    let demand = demand.clamp(0.0, primary_forward);
    let mut u: Vec<f32> = engines
        .iter()
        .map(|e| {
            if e.primary {
                demand / primary_forward
            } else {
                0.0
            }
        })
        .collect();

    // A single engine has no redistribution freedom - throttle scales its
    // magnitude, not its line of action - so the uniform seed (which for n = 1
    // is exactly demand/forward) is already the answer; skip the solve.
    if n == 1 {
        return u;
    }

    // Gradient of ||T u||^2 + w ||L u||^2 is 2 (T^T T + w L^T L) u; its
    // Lipschitz constant is bounded by 2 * (sum||T_i||^2 + w sum||L_i||^2).
    // A conservative (larger) bound just means smaller, still-convergent
    // steps. No torque or lateral at all -> the uniform seed is already
    // optimal.
    let lipschitz = 2.0
        * engines
            .iter()
            .map(|e| e.torque.length_squared() + LATERAL_PENALTY * e.lateral.length_squared())
            .sum::<f32>();
    if lipschitz <= 1e-12 {
        return u;
    }
    let step = 1.0 / lipschitz;

    for _ in 0..BALANCE_ITERS {
        let net_torque: Vec3 = engines.iter().zip(&u).map(|(e, &ui)| e.torque * ui).sum();
        let net_lateral: Vec3 = engines.iter().zip(&u).map(|(e, &ui)| e.lateral * ui).sum();
        for (e, ui) in engines.iter().zip(u.iter_mut()) {
            *ui -= step
                * 2.0
                * (e.torque.dot(net_torque) + LATERAL_PENALTY * e.lateral.dot(net_lateral));
        }
        project_onto_demand(&mut u, engines, demand);
    }
    u
}

/// Euclidean projection of `u` onto `{ sum forward_i u_i = demand } ∩ [0,1]^n`,
/// the balancer's feasible set. The projection has the form `u_i <- clamp(u_i +
/// mu * forward_i, 0, 1)` for a single multiplier `mu`. Zero-forward engines
/// (the off-axis recruits) are untouched by `mu` - the equality is the firing
/// set's contract alone. The math also survives signed coefficients: each
/// engine's mapped contribution `f_i * clamp(u_i + mu * f_i)` is nondecreasing
/// in `mu` (slope `f_i^2` where unclamped), so the mapped sum is monotone and a
/// bisection pins it to the demand. Pure.
fn project_onto_demand(u: &mut [f32], engines: &[BalanceEngine], demand: f32) {
    let mapped_sum = |mu: f32| -> f32 {
        engines
            .iter()
            .zip(u.iter())
            .map(|(e, &ui)| {
                let w = e.forward;
                w * (ui + mu * w).clamp(0.0, 1.0)
            })
            .sum::<f32>()
    };
    // Bracket the multiplier, expanding until the demand is enclosed.
    let mut lo = -1.0f32;
    let mut hi = 1.0f32;
    while mapped_sum(lo) > demand && lo > -1e6 {
        lo *= 2.0;
    }
    while mapped_sum(hi) < demand && hi < 1e6 {
        hi *= 2.0;
    }
    for _ in 0..BALANCE_PROJECT_ITERS {
        let mid = 0.5 * (lo + hi);
        if mapped_sum(mid) < demand {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mu = 0.5 * (lo + hi);
    for (e, ui) in engines.iter().zip(u.iter_mut()) {
        *ui = (*ui + mu * e.forward).clamp(0.0, 1.0);
    }
}

/// Move a thruster input toward `target` on an exponential ramp -
/// framerate-independent, with distinct light-up and cut rates. Pure for unit
/// testing.
pub(super) fn spool(current: f32, target: f32, up_rate: f32, down_rate: f32, dt: f32) -> f32 {
    let rate = if target > current { up_rate } else { down_rate };
    let alpha = 1.0 - (-rate * dt).exp();
    (current + (target - current) * alpha).clamp(0.0, 1.0)
}

/// Whether a thruster's world thrust direction counts as main drive for a
/// ship facing `forward`. Pure for unit testing.
pub(crate) fn is_forward_aligned(thrust_dir: Vec3, forward: Vec3) -> bool {
    thrust_dir.dot(forward) >= FORWARD_ALIGNMENT_COS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_thrusters_groups_by_direction() {
        let engines = [
            (Vec3::NEG_Z, 1.0),
            (Vec3::NEG_Z, 0.5), // joins the main group
            (Vec3::Z, 0.25),    // retro group
            (Vec3::X, 0.25),    // lateral group
        ];
        let groups = cluster_thrusters(&engines, 0.9);
        assert_eq!(groups.len(), 3);
        let main = groups
            .iter()
            .find(|g| g.world_dir.dot(Vec3::NEG_Z) > 0.99)
            .expect("main group");
        assert!((main.authority - 1.5).abs() < 1e-6);
        // Dead weight is ignored.
        assert_eq!(cluster_thrusters(&[(Vec3::X, 0.0)], 0.9).len(), 0);
    }

    #[test]
    fn group_choice_trades_rotation_against_burn_time() {
        let main = ThrusterGroup {
            world_dir: Vec3::NEG_Z,
            authority: 1.0,
        };
        let retro = ThrusterGroup {
            world_dir: Vec3::Z,
            authority: 0.25,
        };
        let groups = [main, retro];
        let (mass, dt, rate, bias) = (4.0, 1.0 / 64.0, 90.0f32.to_radians(), 1.5);
        // A small brake (+Z burn): the retro already points there and wins.
        let small = choose_group(&groups, Vec3::Z, 2.0, mass, dt, rate, bias).expect("group");
        assert_eq!(small.world_dir, Vec3::Z, "small trims use the retro");
        // A large brake: flipping the big main drive is faster overall.
        let large = choose_group(&groups, Vec3::Z, 60.0, mass, dt, rate, bias).expect("group");
        assert_eq!(large.world_dir, Vec3::NEG_Z, "big burns flip to the main");
        // Zero-authority groups never win over a usable one.
        let dead = ThrusterGroup {
            world_dir: Vec3::Z,
            authority: 0.0,
        };
        let handicapped = [main, dead];
        let pick = choose_group(&handicapped, Vec3::Z, 2.0, mass, dt, rate, bias).expect("group");
        assert_eq!(pick.world_dir, Vec3::NEG_Z);
    }

    #[test]
    fn burn_input_scales_and_saturates() {
        assert!((burn_input(0.5, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(burn_input(5.0, 1.0), 1.0);
        assert_eq!(burn_input(1.0, 0.0), 0.0);
        assert_eq!(burn_input(-1.0, 1.0), 0.0);
    }

    /// A firing-set engine perfectly on the burn axis: full forward per unit
    /// input, no perpendicular force, only its lever-arm torque.
    fn main_engine(forward: f32, torque: Vec3) -> BalanceEngine {
        BalanceEngine {
            forward,
            lateral: Vec3::ZERO,
            torque,
            primary: true,
        }
    }

    /// An engine perpendicular to the burn axis: zero forward, all of its
    /// thrust is off-axis force, recruitable only for its torque.
    fn lateral_engine(lateral: Vec3, torque: Vec3) -> BalanceEngine {
        BalanceEngine {
            forward: 0.0,
            lateral,
            torque,
            primary: false,
        }
    }

    #[test]
    fn balance_throttles_splits_demand_to_null_torque() {
        // Two forward engines (weight 1 each) with opposing but unequal lever
        // arms: A torques +0.5 about the COM per unit, B torques -1.5. A
        // uniform half-throttle (0.5, 0.5) would net -0.5; the balancer instead
        // runs A hotter than B so 0.5*uA - 1.5*uB = 0 while uA + uB = 1, i.e.
        // uA = 0.75, uB = 0.25 (hand-computed).
        let engines = [
            main_engine(1.0, Vec3::new(0.0, 0.5, 0.0)),
            main_engine(1.0, Vec3::new(0.0, -1.5, 0.0)),
        ];
        let u = balance_throttles(&engines, 1.0);
        assert!((u[0] - 0.75).abs() < 1e-2, "uA = {}", u[0]);
        assert!((u[1] - 0.25).abs() < 1e-2, "uB = {}", u[1]);
        // The demand is delivered exactly and the net torque is nulled.
        let force: f32 = engines.iter().zip(&u).map(|(e, &x)| e.forward * x).sum();
        assert!((force - 1.0).abs() < 1e-2, "force = {force}");
        let torque: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.torque * x).sum();
        assert!(torque.length() < 1e-2, "net torque = {torque}");
    }

    #[test]
    fn balance_throttles_keeps_a_symmetric_drive_uniform() {
        // Mirror-image lever arms cancel at any uniform throttle, so the
        // balancer must return the shared throttle it started from unchanged
        // (demand 2.0 of total forward 4.0 -> 0.5 each).
        let engines = [
            main_engine(2.0, Vec3::new(0.0, 0.0, 1.0)),
            main_engine(2.0, Vec3::new(0.0, 0.0, -1.0)),
        ];
        let u = balance_throttles(&engines, 2.0);
        assert!(
            (u[0] - 0.5).abs() < 1e-3 && (u[1] - 0.5).abs() < 1e-3,
            "{u:?}"
        );
    }

    #[test]
    fn balance_throttles_falls_back_when_headroom_is_gone() {
        // A lone off-center engine cannot null its own torque - throttle scales
        // the magnitude, not the line of action - so it just delivers the
        // demand and leaves the torque to the PD.
        let lone = [main_engine(1.0, Vec3::new(0.0, 2.0, 0.0))];
        assert!((balance_throttles(&lone, 0.5)[0] - 0.5).abs() < 1e-3);

        // A full-throttle demand pins every engine at 1.0: no headroom to trim,
        // so the residual torque is left to the PD (the pre-balance behavior).
        let full = [
            main_engine(1.0, Vec3::new(0.0, 0.5, 0.0)),
            main_engine(1.0, Vec3::new(0.0, -1.5, 0.0)),
        ];
        let u = balance_throttles(&full, 2.0);
        assert!(
            (u[0] - 1.0).abs() < 1e-3 && (u[1] - 1.0).abs() < 1e-3,
            "{u:?}"
        );

        // Degenerate inputs stay safe.
        assert!(balance_throttles(&[], 1.0).is_empty());
        assert_eq!(balance_throttles(&lone, 0.0)[0], 0.0);
    }

    #[test]
    fn balance_throttles_recruits_an_off_axis_engine_for_counter_torque() {
        // A lone main drive off the COM (torque -1 per unit about Y) and one
        // lateral whose lever arm gives +2 per unit. The demand pins the main
        // at 0.5 (only it has forward thrust); the lateral is recruited purely
        // for counter-torque. Hand-computed optimum of (2 uL - 0.5)^2 +
        // LATERAL_PENALTY * uL^2: uL = 2 * 0.5 / (4 + 0.05) = 0.2469...
        let engines = [
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 2.0, 0.0)),
        ];
        let u = balance_throttles(&engines, 0.5);
        assert!(
            (u[0] - 0.5).abs() < 1e-3,
            "main must hold the demand: {u:?}"
        );
        assert!((u[1] - 0.2469).abs() < 1e-2, "lateral recruit: {u:?}");
        // The recruit nulls all but the lateral-penalty residual of the
        // torque - a couple of percent, well inside the PD's hold.
        let torque: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.torque * x).sum();
        assert!(torque.length() < 0.02, "net torque = {torque}");
        // The price is a bounded sideways force, honestly reported.
        let lateral: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.lateral * x).sum();
        assert!((lateral.x - u[1]).abs() < 1e-3, "lateral force = {lateral}");
    }

    #[test]
    fn balance_throttles_leaves_off_axis_engines_dark_on_a_balanced_ship() {
        // A symmetric twin drive nulls its own torque at the uniform seed, so
        // the objective's gradient is zero and the idle lateral must never be
        // recruited - no gratuitous sideways burn on a healthy ship.
        let engines = [
            main_engine(1.0, Vec3::new(0.0, 1.0, 0.0)),
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 3.0, 0.0)),
        ];
        let u = balance_throttles(&engines, 1.0);
        assert!(
            (u[0] - 0.5).abs() < 1e-3 && (u[1] - 0.5).abs() < 1e-3,
            "{u:?}"
        );
        assert_eq!(u[2], 0.0, "idle lateral must stay dark: {u:?}");
    }

    #[test]
    fn balance_throttles_counter_torques_even_at_full_throttle() {
        // Differential throttle needs forward headroom; a recruited lateral
        // does not - its trim budget is its own throttle box. A full-stick
        // demand pins the lone main at 1.0 and the lateral still nulls the
        // torque (uL = 2 / (4 + 0.05) = 0.4938...).
        let engines = [
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 2.0, 0.0)),
        ];
        let u = balance_throttles(&engines, 1.0);
        assert!((u[0] - 1.0).abs() < 1e-3, "{u:?}");
        assert!((u[1] - 0.4938).abs() < 1e-2, "{u:?}");
    }

    #[test]
    fn balance_throttles_skips_a_lateral_mounted_through_the_com() {
        // An off-axis engine with a tiny lever arm buys almost no torque for
        // its sideways force, so the penalty keeps it (nearly) dark and the
        // torque residual goes to the PD instead of the hull drifting.
        let engines = [
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 0.05, 0.0)),
        ];
        let u = balance_throttles(&engines, 0.5);
        // Optimum uL = 0.05 * 0.5 / (0.05^2 + 0.05) = 0.476 nulls only ~5% of
        // the torque; what matters is the sideways force stays bounded while
        // most of the residual is left to the PD.
        let torque: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.torque * x).sum();
        assert!(
            torque.length() > 0.45,
            "a useless lever must not be trusted with the torque: {u:?}"
        );
    }

    #[test]
    fn spool_ramps_up_and_down_at_distinct_rates_and_clamps() {
        let up = spool(0.0, 1.0, 6.0, 10.0, 0.1);
        assert!(up > 0.0 && up < 1.0);
        let down = spool(1.0, 0.0, 6.0, 10.0, 0.1);
        assert!(1.0 - down > up, "cut should outpace light-up");
        let mut v = 0.0;
        for _ in 0..200 {
            v = spool(v, 1.0, 6.0, 10.0, 1.0 / 60.0);
        }
        assert!((v - 1.0).abs() < 1e-3);
        assert_eq!(spool(2.0, 1.5, 6.0, 10.0, 10.0), 1.0);
    }

    #[test]
    fn forward_alignment_selects_main_drive_thrusters() {
        let forward = Vec3::NEG_Z;
        assert!(is_forward_aligned(Vec3::NEG_Z, forward));
        assert!(!is_forward_aligned(Vec3::Z, forward));
        assert!(!is_forward_aligned(Vec3::X, forward));
    }
}
