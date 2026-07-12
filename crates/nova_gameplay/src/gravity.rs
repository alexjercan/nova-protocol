//! Nova's gravity layer: authored one-way gravity wells with a sphere of
//! influence (patched-conics-lite).
//!
//! Design: docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md.
//! Designated bodies carry a [`GravityWell`]; entities opt in via
//! [`GravityAffected`] and feel the real inverse square `a = mu / r^2` toward
//! the well's center - clamped to the surface value below
//! `body_radius + surface_margin` (no singularity slingshots), smoothstep-
//! faded to zero over the outer band of the SOI (no force step at the
//! boundary), exactly zero outside. When SOIs overlap only the dominant well
//! (strongest pull at the entity's position, with hysteresis) applies, so an
//! entity is always in exactly one body's orbit or in flat space.
//!
//! Gravity here is one-way by construction: wells pull only opted-in
//! entities, a well source never opts in (the force system additionally
//! filters wells out of the affected set), and strength is authored
//! ([`GravityWell::from_surface_gravity`], capped by
//! [`GravitySettings::max_surface_gravity`]) rather than mass-derived. That
//! is what makes the world provably unable to clump: rocks do not pull
//! rocks, and no well can out-muscle a live main drive.
//!
//! The math lives in pure helpers ([`well_accel`], [`circular_orbit_speed`],
//! [`dominant_well`]) so the well-force core stays game-agnostic - a future
//! bevy_common_systems promotion candidate - and so the ORBIT autopilot verb
//! (task 20260709-193339) can plan with the same formulas the force system
//! integrates.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        circular_orbit_speed, dominant_well, well_accel, DominantWell, GravityAffected,
        GravitySettings, GravityWell, NovaGravityPlugin, NovaGravitySystems,
    };
}

/// A gravity well on a designated body. The well pulls [`GravityAffected`]
/// entities toward this entity's position; it never pulls other wells.
///
/// Prefer [`GravityWell::from_surface_gravity`] over filling the fields by
/// hand - it derives `mu` and the SOI from the authored surface gravity and
/// body radius and applies the strength cap.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct GravityWell {
    /// Gravitational parameter, u^3/s^2: `a = mu / r^2`. Authored via
    /// `surface_gravity * body_radius^2`, never derived from collider mass
    /// (true gravity at game scale is unplayably weak, so any orbit-capable
    /// strength is a designer stat).
    pub mu: f32,
    /// Nominal radius of the body, world units. The pull is clamped to its
    /// surface value below `body_radius + surface_margin`.
    pub body_radius: f32,
    /// Sphere-of-influence radius, world units. The pull fades to exactly
    /// zero at this distance and stays zero beyond it.
    pub soi_radius: f32,
}

impl GravityWell {
    /// Build a well from an authored surface gravity (u/s^2 at
    /// `body_radius`). The strength is clamped to
    /// [`GravitySettings::max_surface_gravity`] - the guardrail that keeps
    /// every well escapable under main drive - and the SOI derives from the
    /// body radius via [`GravitySettings::soi_factor`].
    pub fn from_surface_gravity(
        surface_gravity: f32,
        body_radius: f32,
        settings: &GravitySettings,
    ) -> Self {
        let g = surface_gravity.clamp(0.0, settings.max_surface_gravity);
        Self {
            mu: g * body_radius * body_radius,
            body_radius,
            soi_radius: settings.soi_factor * body_radius,
        }
    }
}

/// Opt-in marker: only entities carrying this feel gravity wells. Inserted
/// automatically on ship roots (player and AI - one arena, one physics),
/// torpedo projectiles, and turret rounds. Turret rounds opted in as of the
/// bullet-gravity spike (docs/spikes/20260712-112113): the curve is only
/// perceptible on close grazing passes, but since the target ship already
/// feels gravity, letting rounds fall too is a free common-mode correction to
/// the lead solver's aim-behind-a-falling-target miss. Section debris still
/// skips (perf; a later flourish). Never insert this on a well source.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct GravityAffected;

/// The well that currently owns this entity (strongest pull, with
/// hysteresis). Present only while inside at least one SOI; the force system
/// maintains it, and an `On<Remove, GravityWell>` observer strips it the
/// moment the owned well dies, so consumers never see a dangling entity for
/// longer than the current flush (still: handle a failed `Query::get`
/// gracefully). The HUD and the ORBIT verb read this to know *whose* orbit
/// the ship is in. Not entity-mapped for reflection: scene serialization
/// would not remap the id (no current consumer does this).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Deref, Reflect)]
#[reflect(Component)]
pub struct DominantWell(pub Entity);

/// All gravity tunables in one reflected resource, for the inspector and a
/// future settings menu. Per-body strength is authored on the scenario side
/// (see `AsteroidConfig`); these are the global defaults and guardrails.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct GravitySettings {
    /// Surface gravity a designated body gets when the scenario does not
    /// author one, u/s^2 at the body's nominal radius. 6.0 on a 20u rock
    /// gives mu = 2400: v_circ ~ 6.9 u/s at r = 50u with a ~45s lap.
    /// Doubled from 3.0 after the 2026-07-10 playtest ("a bit weak...
    /// I would like it to be stronger so you actually feel it") - the
    /// arrival solver budgets the pull since 20260710-193500, so a
    /// stronger well costs an earlier flip, not a crash.
    pub default_surface_gravity: f32,
    /// Bodies below this nominal radius (world units) get no well by default;
    /// the 1-3u field rocks stay flat space. A scenario can still author a
    /// well onto a small body explicitly.
    pub min_well_radius: f32,
    /// SOI radius as a multiple of the body radius. 8.0 puts a 20u rock's
    /// SOI at 160u: the fun orbit band (30-80u) sits deep in the unfaded
    /// core, and the well announces itself with a gentle inverse-square tug
    /// long before the rock fills the screen. Retuned from 4.0 after the
    /// 2026-07-10 playtest ("had to be almost near it to experience the
    /// pull").
    pub soi_factor: f32,
    /// Fraction of the SOI (outermost band) over which the pull smoothsteps
    /// to zero, so there is no force discontinuity at the boundary for the
    /// autopilot to chatter on. Orbits are only trusted inside the unfaded
    /// core; the ORBIT verb clamps its target radius into that band.
    pub fade_fraction: f32,
    /// The pull is clamped to its surface value below
    /// `body_radius + surface_margin` (world units), so grazing the rock is
    /// a bump, never a singularity slingshot.
    pub surface_margin: f32,
    /// A challenger well takes ownership only when its pull beats the
    /// incumbent's by this factor (> 1.0), so SOI-boundary flicker cannot
    /// flip wells tick to tick. In dense fields this degrades to "nearest
    /// big rock wins", which is predictable and readable.
    pub switch_hysteresis: f32,
    /// Hard cap on authored surface gravity, u/s^2 - the "gravity never
    /// out-muscles a live ship" guardrail. This is a tuning contract, not
    /// enforced against the emergent ship acceleration (which comes from
    /// live thruster magnitudes over live mass): for scale, the flight
    /// tests' minimal ship accelerates at ~21 u/s^2 (magnitude 1.0 impulse
    /// per 1/64s tick over mass 3), and shipped builds are the same order.
    /// Keep this well under the weakest flyable build when retuning.
    /// Raised 5.0 -> 10.0 with the 2026-07-10 strength retune (still
    /// under half the reference ship's authority; the gravity-aware
    /// arrival degrades to an explicit no-stopping-plan state rather
    /// than crashing if a build ever dips below it).
    pub max_surface_gravity: f32,
}

impl Default for GravitySettings {
    fn default() -> Self {
        Self {
            default_surface_gravity: 6.0,
            min_well_radius: 5.0,
            soi_factor: 8.0,
            fade_fraction: 0.15,
            surface_margin: 1.0,
            switch_hysteresis: 1.1,
            max_surface_gravity: 10.0,
        }
    }
}

/// System set for the gravity layer; ordered before the section systems in
/// `FixedUpdate`, alongside the flight layer, so the well pull lands in the
/// same physics step as the tick's thruster impulses.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaGravitySystems;

/// Plugin wiring the gravity layer.
#[derive(Default)]
pub struct NovaGravityPlugin;

impl Plugin for NovaGravityPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaGravityPlugin: build");

        app.init_resource::<GravitySettings>()
            // Register the whole reflected tree, not just the resource root.
            .register_type::<GravitySettings>()
            .register_type::<GravityWell>()
            .register_type::<GravityAffected>()
            .register_type::<DominantWell>();

        app.add_observer(insert_gravity_affected_on_ship);
        app.add_observer(insert_gravity_affected_on_torpedo);
        app.add_observer(insert_gravity_affected_on_turret_round);
        app.add_observer(remove_dominant_well_on_well_removed);

        app.configure_sets(
            FixedUpdate,
            NovaGravitySystems.before(SpaceshipSectionSystems),
        );
        app.add_systems(FixedUpdate, gravity_well_system.in_set(NovaGravitySystems));
    }
}

/// Ship roots opt into gravity - player and AI alike, one arena, one physics.
fn insert_gravity_affected_on_ship(add: On<Add, SpaceshipRootMarker>, mut commands: Commands) {
    commands.entity(add.entity).try_insert(GravityAffected);
}

/// Torpedoes opt in too: PN guidance is closed-loop on line-of-sight rate,
/// so it self-corrects through wells (spike decision 5).
fn insert_gravity_affected_on_torpedo(
    add: On<Add, TorpedoProjectileMarker>,
    mut commands: Commands,
) {
    commands.entity(add.entity).try_insert(GravityAffected);
}

/// Turret rounds opt in as of the bullet-gravity spike
/// (docs/spikes/20260712-112113). They ride the same `gravity_well_system` as
/// ships and torpedoes - it only touches `DominantWell` on an SOI crossing
/// (~twice over a round's life, not per tick), so ~500 live rounds/turret is
/// affordable; a lighter no-`DominantWell` path was left unbuilt because the
/// measurement did not call for it.
fn insert_gravity_affected_on_turret_round(
    add: On<Add, TurretBulletProjectileMarker>,
    mut commands: Commands,
) {
    commands.entity(add.entity).try_insert(GravityAffected);
}

/// When a well dies (the designated asteroid was destroyed), strip its
/// [`DominantWell`] handles immediately instead of leaving consumers (HUD,
/// ORBIT verb) a dangling entity until the force system's next tick.
fn remove_dominant_well_on_well_removed(
    remove: On<Remove, GravityWell>,
    mut commands: Commands,
    q_dominant: Query<(Entity, &DominantWell)>,
) {
    for (entity, dominant) in &q_dominant {
        if **dominant == remove.entity {
            commands.entity(entity).remove::<DominantWell>();
        }
    }
}

/// Acceleration magnitude of a well at distance `r` from its center: the
/// real inverse square `mu / r^2`, clamped to its surface value below
/// `body_radius + surface_margin`, multiplied by a smoothstep fade over the
/// outer `fade_fraction` of the SOI so the pull reaches exactly zero at the
/// boundary, and exactly zero at and beyond `soi_radius`.
pub fn well_accel(
    mu: f32,
    r: f32,
    body_radius: f32,
    soi_radius: f32,
    fade_fraction: f32,
    surface_margin: f32,
) -> f32 {
    if mu <= 0.0 || soi_radius <= 0.0 || r >= soi_radius {
        return 0.0;
    }

    // Clamp below the surface: grazing the rock is a bump, not a slingshot.
    let r_eff = r.max((body_radius + surface_margin).max(f32::EPSILON));
    let base = mu / (r_eff * r_eff);

    // Smoothstep from 1 at the start of the fade band down to 0 at the SOI
    // edge; 1 everywhere inside the unfaded core.
    let fade_start = soi_radius * (1.0 - fade_fraction.clamp(0.0, 1.0));
    let fade = if r <= fade_start {
        1.0
    } else {
        let t = ((soi_radius - r) / (soi_radius - fade_start).max(f32::EPSILON)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };

    base * fade
}

/// Speed of a circular orbit at radius `r` in a well of parameter `mu`:
/// `v = sqrt(mu / r)`. The ORBIT verb burns to this tangentially.
pub fn circular_orbit_speed(mu: f32, r: f32) -> f32 {
    if mu <= 0.0 || r <= 0.0 {
        return 0.0;
    }
    (mu / r).sqrt()
}

/// Pick the well that owns an entity from `candidates` (each a well entity
/// and its pull at the entity's position, only entries with positive pull),
/// keeping the `current` incumbent unless a challenger beats its pull by the
/// `switch_hysteresis` factor. This is what stops SOI-boundary flicker from
/// flipping ownership tick to tick.
pub fn dominant_well(
    current: Option<Entity>,
    candidates: &[(Entity, f32)],
    switch_hysteresis: f32,
) -> Option<Entity> {
    let (strongest, strongest_pull) = candidates
        .iter()
        .copied()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))?;

    if let Some(incumbent) = current {
        if let Some((_, incumbent_pull)) = candidates
            .iter()
            .copied()
            .find(|(entity, _)| *entity == incumbent)
        {
            if strongest_pull <= incumbent_pull * switch_hysteresis.max(1.0) {
                return Some(incumbent);
            }
        }
    }

    Some(strongest)
}

/// The one force system: every `FixedUpdate` tick, each [`GravityAffected`]
/// entity finds the wells whose SOI contains it, keeps the dominant one
/// (with hysteresis, tracked in [`DominantWell`]), and feels its pull as a
/// central linear acceleration - mass-independent, exactly like gravity, and
/// torque-free (ships are point masses to the well).
///
/// `Without<GravityWell>` on the affected query is the belt-and-braces half
/// of "wells never pull wells": even a misconfigured entity carrying both
/// components feels nothing.
pub(crate) fn gravity_well_system(
    settings: Res<GravitySettings>,
    mut commands: Commands,
    q_wells: Query<(Entity, &Position, &GravityWell)>,
    mut q_affected: Query<
        (Entity, &Position, Option<&DominantWell>, Forces),
        (With<GravityAffected>, Without<GravityWell>),
    >,
    // Reused across all affected entities each tick so the per-entity work is
    // O(wells) with no heap allocation. `clear()` keeps the capacity, so after
    // the first tick these never allocate. This is what makes a PDC's worth of
    // gravity-affected rounds (thousands of affected bodies) affordable on the
    // shared force path: the Vec-per-entity version cost ~2.2 ms/tick over 1500
    // bodies; reusing the buffers drops the whole gravity system's marginal
    // cost to ~0.1 ms/tick (see `gravity_system_marginal_cost`).
    mut candidates: Local<Vec<(Entity, f32, Vec3)>>,
    mut pulls: Local<Vec<(Entity, f32)>>,
) {
    // O(wells x affected) at nova's scale (a handful of wells, tens to a few
    // thousand affected bodies once turret rounds opt in).
    for (entity, position, current, mut forces) in &mut q_affected {
        candidates.clear();
        for (well_entity, well_position, well) in &q_wells {
            // Direction first: freshly spawned bodies sit at avian's
            // Position::PLACEHOLDER (Vector::MAX) until the first physics
            // sync, which makes two same-flush spawns coincident - a
            // degenerate or non-finite offset is not a candidate, so no
            // spurious DominantWell flashes on scenario start.
            let offset = **well_position - **position;
            let r = offset.length();
            let Some(toward_center) = offset.try_normalize() else {
                continue;
            };
            let accel = well_accel(
                well.mu,
                r,
                well.body_radius,
                well.soi_radius,
                settings.fade_fraction,
                settings.surface_margin,
            );
            if accel > 0.0 {
                candidates.push((well_entity, accel, toward_center));
            }
        }

        pulls.clear();
        pulls.extend(
            candidates
                .iter()
                .map(|&(well_entity, accel, _)| (well_entity, accel)),
        );
        let chosen = dominant_well(current.map(|d| **d), &pulls, settings.switch_hysteresis);

        let Some(owner) = chosen else {
            if current.is_some() {
                commands.entity(entity).remove::<DominantWell>();
            }
            continue;
        };

        if current.map(|d| **d) != Some(owner) {
            commands.entity(entity).try_insert(DominantWell(owner));
        }

        // Apply only the dominant well's pull (one orbit or flat space,
        // never a blended field). dominant_well only returns candidate
        // entities, so the find cannot miss; the else is defensive.
        let Some(&(_, accel, toward_center)) = candidates
            .iter()
            .find(|(well_entity, _, _)| *well_entity == owner)
        else {
            continue;
        };
        forces.apply_linear_acceleration(toward_center * accel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pure helpers -----------------------------------------------------

    /// The spike's sanity rock: 20u radius at surface gravity 3 u/s^2.
    const MU: f32 = 1200.0;
    const BODY: f32 = 20.0;
    const SOI: f32 = 80.0;
    const FADE: f32 = 0.15;
    const MARGIN: f32 = 1.0;

    fn accel_at(r: f32) -> f32 {
        well_accel(MU, r, BODY, SOI, FADE, MARGIN)
    }

    #[test]
    fn well_accel_is_inverse_square_in_the_unfaded_core() {
        // Fade band starts at 0.85 * 80 = 68; below it the real formula holds.
        assert_eq!(accel_at(50.0), MU / 2500.0);
        assert_eq!(accel_at(68.0), MU / (68.0 * 68.0));
    }

    #[test]
    fn well_accel_clamps_to_the_surface_value_below_the_margin() {
        let surface = MU / (21.0 * 21.0);
        assert_eq!(accel_at(21.0), surface);
        assert_eq!(accel_at(5.0), surface, "no singularity slingshot");
        assert_eq!(accel_at(0.0), surface);
    }

    #[test]
    fn well_accel_fades_to_exactly_zero_at_the_soi_edge_and_beyond() {
        assert_eq!(accel_at(SOI), 0.0);
        assert_eq!(accel_at(SOI + 20.0), 0.0);
        // Inside the band the pull is positive but below the raw formula,
        // and it shrinks toward the edge.
        let mid_band = accel_at(74.0);
        assert!(mid_band > 0.0 && mid_band < MU / (74.0 * 74.0));
        assert!(accel_at(79.0) < mid_band);
        // Just inside the edge it is a whisker above zero, not a step.
        assert!(accel_at(79.99) < 0.001);
    }

    #[test]
    fn circular_orbit_speed_matches_the_spike_sanity_math() {
        // 20u rock, surface gravity 3: v_circ ~ 4.9 u/s at r = 50u.
        let v = circular_orbit_speed(MU, 50.0);
        assert!((v - 4.898979).abs() < 1e-4, "got {v}");
        assert_eq!(circular_orbit_speed(MU, 0.0), 0.0);
        assert_eq!(circular_orbit_speed(0.0, 50.0), 0.0);
    }

    #[test]
    fn from_surface_gravity_derives_mu_and_soi_and_applies_the_cap() {
        let settings = GravitySettings::default();
        let well = GravityWell::from_surface_gravity(3.0, 20.0, &settings);
        assert_eq!(well.mu, 1200.0);
        assert_eq!(well.body_radius, 20.0);
        assert_eq!(well.soi_radius, settings.soi_factor * 20.0);

        // Authored strength beyond the guardrail is capped, not honored.
        let capped = GravityWell::from_surface_gravity(50.0, 20.0, &settings);
        assert_eq!(capped.mu, settings.max_surface_gravity * 400.0);
    }

    #[test]
    fn dominant_well_keeps_the_incumbent_inside_the_hysteresis_margin() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();

        assert_eq!(dominant_well(None, &[], 1.1), None);
        // No incumbent: strongest wins outright.
        assert_eq!(dominant_well(None, &[(a, 0.3), (b, 0.31)], 1.1), Some(b));
        // Incumbent holds while the challenger is within 10%.
        assert_eq!(dominant_well(Some(a), &[(a, 0.3), (b, 0.31)], 1.1), Some(a));
        // Challenger beats the margin: ownership flips.
        assert_eq!(dominant_well(Some(a), &[(a, 0.3), (b, 0.34)], 1.1), Some(b));
        // Incumbent no longer a candidate (left its SOI): strongest wins.
        assert_eq!(dominant_well(Some(a), &[(b, 0.01)], 1.1), Some(b));
    }

    // --- Observer wiring ---------------------------------------------------

    #[test]
    fn ships_torpedoes_and_turret_rounds_opt_into_gravity() {
        let mut app = App::new();
        app.add_observer(insert_gravity_affected_on_ship);
        app.add_observer(insert_gravity_affected_on_torpedo);
        app.add_observer(insert_gravity_affected_on_turret_round);

        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let torpedo = app.world_mut().spawn(TorpedoProjectileMarker).id();
        let round = app.world_mut().spawn(TurretBulletProjectileMarker).id();
        app.update();

        assert!(app.world().get::<GravityAffected>(ship).is_some());
        assert!(app.world().get::<GravityAffected>(torpedo).is_some());
        assert!(app.world().get::<GravityAffected>(round).is_some());
    }

    // --- Physics-level integration ------------------------------------------
    //
    // A real avian world with the real force system: well pull -> velocity ->
    // orbit. No thrusters anywhere; these cover the substrate alone.

    use crate::integrity::test_support::{settle, unfinished_integrity_physics_app};

    /// The real plugin on the physics harness, so these tests cover the
    /// wiring (observers, resource, set, system) and not just the system fn.
    fn gravity_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(NovaGravityPlugin);
        app.finish();
        app
    }

    fn spawn_well(app: &mut App, position: Vec3) -> Entity {
        let well = GravityWell::from_surface_gravity(3.0, BODY, &GravitySettings::default());
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Transform::from_translation(position),
                well,
            ))
            .id()
    }

    fn spawn_probe(app: &mut App, position: Vec3, velocity: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(position),
                Collider::sphere(0.5),
                ColliderDensity(1.0),
                LinearVelocity(velocity),
                GravityAffected,
            ))
            .id()
    }

    fn position_of(app: &App, entity: Entity) -> Vec3 {
        **app.world().get::<Position>(entity).unwrap()
    }

    fn velocity_of(app: &App, entity: Entity) -> Vec3 {
        **app.world().get::<LinearVelocity>(entity).unwrap()
    }

    #[test]
    fn a_body_seeded_at_v_circ_keeps_a_bounded_orbit() {
        let mut app = gravity_app();
        spawn_well(&mut app, Vec3::ZERO);
        let r0 = 50.0;
        let v = circular_orbit_speed(MU, r0);
        let probe = spawn_probe(&mut app, Vec3::new(r0, 0.0, 0.0), Vec3::new(0.0, 0.0, -v));
        settle(&mut app);

        // ~70 seconds of sim - a full ~64s lap - sampling the radius every
        // tick. A real orbit stays inside a tight band around r0; a broken
        // force profile spirals in or flings out within a fraction of that.
        let (mut r_min, mut r_max) = (f32::MAX, f32::MIN);
        for _ in 0..4200 {
            app.update();
            let r = position_of(&app, probe).length();
            r_min = r_min.min(r);
            r_max = r_max.max(r);
        }

        assert!(
            r_min > 0.8 * r0 && r_max < 1.25 * r0,
            "orbit radius drifted out of [{}, {}]: min {r_min}, max {r_max}",
            0.8 * r0,
            1.25 * r0
        );
        assert!(
            app.world().get::<DominantWell>(probe).is_some(),
            "an orbiting body knows whose orbit it is in"
        );
    }

    /// A dynamic body without `GravityAffected`, so an A/B control against a
    /// gravity-affected round on the same tangential pass can prove the marker
    /// is what curves the trajectory. Mirrors `spawn_probe` minus the marker.
    fn spawn_straight_body(app: &mut App, position: Vec3, velocity: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(position),
                Collider::sphere(0.5),
                ColliderDensity(1.0),
                LinearVelocity(velocity),
            ))
            .id()
    }

    #[test]
    fn a_turret_round_curves_under_a_well_and_a_gravity_free_body_does_not() {
        // The bullet-gravity regression (docs/spikes/20260712-112113): a round
        // on a tangential pass past a well must bend toward the center, and an
        // identical body WITHOUT GravityAffected must fly dead straight. This
        // A/Bs the opt-in - delete the marker (or its observer) and the two
        // trajectories coincide, failing the test.
        //
        // Geometry: well at origin (mu 1200, SOI 160, unfaded core out to
        // 0.85*160 = 136u), body starts at x = b = 40u (deep in the core) on
        // the +z side, flying -z at 40 u/s straight past the well. The pull
        // has a -x (toward-center) component that accumulates transverse
        // deflection measured off the entry lane x = b. Raw `Position` is the
        // FixedUpdate-clock readout.
        let b = 40.0;
        let v = 40.0;
        let z0 = 60.0;
        let ticks = 240; // ~4s: z sweeps +60 -> -60 through closest approach.

        let mut app = gravity_app();
        spawn_well(&mut app, Vec3::ZERO);
        let round = spawn_probe(&mut app, Vec3::new(b, 0.0, z0), Vec3::new(0.0, 0.0, -v));
        settle(&mut app);
        for _ in 0..ticks {
            app.update();
        }
        let deflection = b - position_of(&app, round).x; // toward center => +

        let mut ctrl_app = gravity_app();
        spawn_well(&mut ctrl_app, Vec3::ZERO);
        let ctrl = spawn_straight_body(
            &mut ctrl_app,
            Vec3::new(b, 0.0, z0),
            Vec3::new(0.0, 0.0, -v),
        );
        settle(&mut ctrl_app);
        for _ in 0..ticks {
            ctrl_app.update();
        }
        let control_drift = (b - position_of(&ctrl_app, ctrl).x).abs();

        // Measured on this rig: deflection = 3.25u, control drift = 0.0u. The
        // 2.0u threshold sits well above integrator noise (the control is
        // exactly straight) and below the signal; the control assertion is the
        // in-test A/B that fails if the round stops feeling gravity.
        assert!(
            control_drift < 0.05,
            "a gravity-free body must fly straight; drifted {control_drift}u"
        );
        assert!(
            deflection > 2.0,
            "the round should curve toward the well; deflection {deflection}u \
             (gravity-free control drifted {control_drift}u)"
        );
    }

    /// Marginal per-tick cost of letting a PDC's worth of rounds feel gravity.
    /// Not a CI gate (wall-clock is environment-dependent, hence `#[ignore]`);
    /// run manually to record the number that justified riding the shared
    /// `gravity_well_system` instead of building a lighter bullet-only path:
    ///
    /// ```text
    /// cargo test -p nova_gameplay gravity_system_marginal_cost -- --ignored --nocapture
    /// ```
    ///
    /// It times N=1500 dynamic bodies (~3 turrets' worth of live rounds) with
    /// the gravity opt-in + a well, against N identical bodies with neither.
    /// Both worlds integrate the same N rigid bodies, so avian's cost cancels
    /// and the delta is the gravity system's own O(wells x affected) work plus
    /// its per-affected Vec alloc.
    ///
    /// Scope: the bodies start INSIDE the SOI, so this is the steady-state
    /// per-tick force cost. It does not isolate the per-crossing `DominantWell`
    /// insert/remove (archetype-move) churn a round pays entering/leaving an
    /// SOI; that is bounded by SOI-crossings/sec (small - a round crosses at
    /// most a couple of boundaries in its whole life) and is not the hot path
    /// this measures.
    #[test]
    #[ignore = "wall-clock perf measurement, run manually"]
    fn gravity_system_marginal_cost() {
        use std::time::Instant;

        const N: usize = 1500;
        const TICKS: usize = 600;

        fn time_ticks(with_gravity: bool) -> f64 {
            let mut app = gravity_app();
            if with_gravity {
                spawn_well(&mut app, Vec3::ZERO);
            }
            // Spread the bodies through the well's core so most are inside an
            // SOI (the expensive case: they push a candidate and hold a
            // DominantWell), on gentle non-colliding lanes.
            for i in 0..N {
                let a = i as f32 * 0.31;
                let pos = Vec3::new(30.0 + (i % 40) as f32, a.sin() * 20.0, a.cos() * 20.0);
                let vel = Vec3::new(0.0, 0.0, -20.0);
                if with_gravity {
                    spawn_probe(&mut app, pos, vel);
                } else {
                    spawn_straight_body(&mut app, pos, vel);
                }
            }
            settle(&mut app);
            let start = Instant::now();
            for _ in 0..TICKS {
                app.update();
            }
            start.elapsed().as_secs_f64() * 1e3 / TICKS as f64 // ms/tick
        }

        let with = time_ticks(true);
        let without = time_ticks(false);
        eprintln!(
            "gravity marginal cost over {N} bodies: with={with:.3} ms/tick, \
             without={without:.3} ms/tick, delta={:.3} ms/tick",
            with - without
        );
    }

    #[test]
    fn despawning_the_owned_well_releases_dominance_without_panic() {
        let mut app = gravity_app();
        let well = spawn_well(&mut app, Vec3::ZERO);
        let r0 = 50.0;
        let v = circular_orbit_speed(MU, r0);
        let probe = spawn_probe(&mut app, Vec3::new(r0, 0.0, 0.0), Vec3::new(0.0, 0.0, -v));
        settle(&mut app);
        app.update();
        assert_eq!(**app.world().get::<DominantWell>(probe).unwrap(), well);

        // The Gravity Rock is destructible; orbiters must survive its death.
        app.world_mut().entity_mut(well).despawn();
        app.update();
        assert!(
            app.world().get::<DominantWell>(probe).is_none(),
            "a dead well must not leave a dangling DominantWell"
        );

        // With the well gone the probe coasts: no force, straight line.
        let coasting = velocity_of(&app, probe);
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(velocity_of(&app, probe), coasting);
    }

    #[test]
    fn a_ship_root_is_pulled_through_the_real_plugin_wiring() {
        let mut app = gravity_app();
        spawn_well(&mut app, Vec3::ZERO);
        // A bare ship root: GravityAffected must arrive via the plugin's
        // observer, not by hand.
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
                Collider::sphere(0.5),
                ColliderDensity(1.0),
                LinearVelocity(Vec3::ZERO),
                SpaceshipRootMarker,
            ))
            .id();
        settle(&mut app);
        for _ in 0..60 {
            app.update();
        }

        assert!(
            app.world().get::<GravityAffected>(ship).is_some(),
            "the plugin's observer must opt ship roots in"
        );
        assert!(
            velocity_of(&app, ship).x < -0.1,
            "the ship must fall toward the well, got {:?}",
            velocity_of(&app, ship)
        );
    }

    #[test]
    fn outside_the_soi_there_is_no_force_at_all() {
        let mut app = gravity_app();
        spawn_well(&mut app, Vec3::ZERO);
        // SOI ends at 160 (soi_factor 8 x 20u body); park at 250 with zero
        // velocity.
        let probe = spawn_probe(&mut app, Vec3::new(250.0, 0.0, 0.0), Vec3::ZERO);
        settle(&mut app);

        for _ in 0..120 {
            app.update();
        }

        assert_eq!(
            velocity_of(&app, probe),
            Vec3::ZERO,
            "flat space must stay flat"
        );
        assert!(app.world().get::<DominantWell>(probe).is_none());
    }

    #[test]
    fn a_body_that_did_not_opt_in_feels_nothing_inside_the_soi() {
        let mut app = gravity_app();
        spawn_well(&mut app, Vec3::ZERO);
        let bystander = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
                Collider::sphere(0.5),
                ColliderDensity(1.0),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        settle(&mut app);

        for _ in 0..120 {
            app.update();
        }

        assert_eq!(velocity_of(&app, bystander), Vec3::ZERO);
    }

    #[test]
    fn wells_never_pull_wells_even_when_misconfigured_as_affected() {
        let mut app = gravity_app();
        let a = spawn_well(&mut app, Vec3::ZERO);
        // B sits deep inside A's SOI and even carries GravityAffected by
        // mistake; the Without<GravityWell> filter must still exclude it.
        let b = spawn_well(&mut app, Vec3::new(60.0, 0.0, 0.0));
        app.world_mut().entity_mut(b).insert((
            RigidBody::Dynamic,
            Collider::sphere(1.0),
            ColliderDensity(1.0),
            GravityAffected,
        ));
        settle(&mut app);

        for _ in 0..120 {
            app.update();
        }

        assert_eq!(velocity_of(&app, b), Vec3::ZERO, "wells must not clump");
        let a_position = position_of(&app, a);
        assert_eq!(a_position, Vec3::ZERO, "static wells stay on rails");
    }

    #[test]
    fn overlapping_sois_hand_off_with_hysteresis_not_flicker() {
        let mut app = gravity_app();
        // Two equal wells 120u apart: with SOI 160 each, their spheres
        // overlap across the whole segment between them.
        let a = spawn_well(&mut app, Vec3::ZERO);
        let b = spawn_well(&mut app, Vec3::new(120.0, 0.0, 0.0));
        // At x = 55 A's pull (r 55) clearly beats B's (r 65): A owns.
        let probe = spawn_probe(&mut app, Vec3::new(55.0, 0.0, 0.0), Vec3::ZERO);
        settle(&mut app);
        app.update();
        assert_eq!(**app.world().get::<DominantWell>(probe).unwrap(), a);

        // At x = 60.5 B is already the stronger well (r 59.5 vs 60.5) but
        // only by ~3% - inside the 10% hysteresis, so A keeps ownership.
        app.world_mut()
            .entity_mut(probe)
            .insert((Position(Vec3::new(60.5, 0.0, 0.0)), LinearVelocity::ZERO));
        app.update();
        assert_eq!(
            **app.world().get::<DominantWell>(probe).unwrap(),
            a,
            "a challenger inside the hysteresis margin must not steal ownership"
        );

        // At x = 65 B beats A by ~40%: ownership flips.
        app.world_mut()
            .entity_mut(probe)
            .insert((Position(Vec3::new(65.0, 0.0, 0.0)), LinearVelocity::ZERO));
        app.update();
        assert_eq!(**app.world().get::<DominantWell>(probe).unwrap(), b);
    }
}
