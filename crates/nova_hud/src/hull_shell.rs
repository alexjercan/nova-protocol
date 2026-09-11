//! What size the directional-HUD shells are: the contract that keeps the
//! velocity sphere and the gravity sphere OUTSIDE the hull they belong to.
//!
//! Both shells used to be authored constants - 50 m and 56 m - which is a
//! promise only a small hull keeps. The industrial carrier is 360 m stem to
//! stern, so a fixed 56 m sphere sits buried inside it with a bulge poking
//! through the keel.
//!
//! The contract here is physical and stated in meters: the velocity shell
//! stands [`HULL_CLEARANCE`] outside the hull's own
//! [`HullEnvelopeRadius`] and the gravity shell stands
//! [`SHELL_SEPARATION`] outside that. Both are FIXED gaps, not fractions of
//! hull size - a shell that scaled with the ship would leave a big hull wearing
//! a sphere metres clear of nothing and reading as a bubble.
//!
//! The eased radius is ONE number per hull, shared by both shells, so their
//! separation cannot drift while a hull that just lost sections converges.
//! Sizing is here; the widget itself, its cone and its shading stay in
//! [`velocity`](super::velocity), and the generic orbit rig stays in
//! `nova_gameplay`.

use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    pbr::ExtendedMaterial,
    prelude::*,
};
use nova_events::units::prelude::*;
use nova_gameplay::transform::prelude::DirectionalSphereOrbit;
use nova_ship::prelude::HullEnvelopeRadius;

use super::velocity::{prelude::*, DirectionSphereMaterial, VelocityHudSphereMarker};

/// The shell contract: its clearances, the per-hull eased radius, the
/// `hull_shell` bundle and `HullShellPlugin`.
pub mod prelude {
    pub use super::{
        hull_shell, outer_shell_radius, shell_radii, HudShellEnvelopes, HullShellClearance,
        HullShellPending, HullShellPlugin, HULL_CLEARANCE, SHELL_SEPARATION,
    };
}

/// How far outside the hull's physical envelope the INNER (velocity) shell
/// stands.
///
/// The envelope is the hull's collider reach; the visual skin, its greebles and
/// the shell's own shading all live in the gap, and the projection has to read
/// as a projection AROUND the ship rather than as paint on it. Five meters does
/// that on a cutter and on a carrier alike, which is the point of a fixed gap.
pub const HULL_CLEARANCE: Meters = Meters(5.0);

/// How far outside the velocity shell the gravity shell stands - the gap the
/// two spheres have always had, kept exact so they never z-fight.
pub const SHELL_SEPARATION: Meters = Meters(6.0);

/// How long a SHRINKING shell takes to cover half the distance to its new
/// radius. Growth is immediate; only shrinking eases, because an expanding
/// shell that lagged would expose the hull it is meant to contain.
///
/// A half-life rather than a fixed-duration transition: repeated damage
/// retargets a half-life mid-flight without restarting a timer, so a hull
/// coming apart section by section converges smoothly instead of stepping.
const SHRINK_HALF_LIFE_SECS: f32 = 0.150;

/// How close the eased radius has to get before it is simply set to the target.
/// An exponential approach never arrives, and a shell forever a fraction of a
/// millimetre off its contract would re-upload its material every frame.
const SHELL_SNAP: Meters = Meters(0.01);

/// The clearance this shell keeps outside the hull's physical envelope. Tagging
/// a directional-HUD widget with it is what puts the widget under this module's
/// sizing; an untagged widget keeps its spawn-time radius.
#[derive(Component, Debug, Clone, Copy, PartialEq, Deref, Reflect)]
#[reflect(Component)]
pub struct HullShellClearance(pub Meters);

/// On a shell that has never seen its hull's envelope. It is held HIDDEN while
/// this is on it: a shell drawn at a guessed radius during the frames a hull is
/// still assembling is exactly the wrong picture to flash.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct HullShellPending;

/// The eased shell envelope of each hull a shell is watching, world units.
///
/// Keyed by HULL rather than held per widget, because the velocity and gravity
/// shells of one ship must derive from the SAME number: eased separately they
/// would converge at their own rates and their authored 6 m separation would
/// breathe while a damaged hull settles.
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct HudShellEnvelopes(EntityHashMap<f32>);

/// The two shell radii, world units, for a hull whose eased envelope is
/// `envelope`: the velocity shell and the gravity shell outside it.
///
/// The one place the contract is arithmetic, so every consumer - the spheres,
/// their cones and the flight chips parked off the outer edge - reads one
/// answer.
pub fn shell_radii(envelope: f32) -> (f32, f32) {
    let velocity = envelope + HULL_CLEARANCE.to_engine();
    (velocity, velocity + SHELL_SEPARATION.to_engine())
}

/// The outer (gravity) shell radius of `hull`, world units, or `None` before
/// its envelope is known.
///
/// Answered even while the gravity shell itself is HIDDEN in flat space: the
/// flight chips park off this edge whether or not the sphere marking it is
/// drawn, so the readouts do not jump when a ship leaves a well.
pub fn outer_shell_radius(envelopes: &HudShellEnvelopes, hull: Entity) -> Option<f32> {
    envelopes
        .get(&hull)
        .map(|&envelope| shell_radii(envelope).1)
}

/// Put a directional-HUD widget under the shell contract at `clearance` outside
/// the hull's physical envelope.
///
/// Spawned alongside [`velocity_hud`], whose config should carry a zero radius:
/// the shell has no size until its hull publishes one, and it is hidden until
/// then.
pub fn hull_shell(clearance: Meters) -> impl Bundle {
    (HullShellClearance(clearance), HullShellPending)
}

/// The frame-rate-independent ease toward `target`: immediate on the way out,
/// a [`SHRINK_HALF_LIFE_SECS`] half-life on the way in, snapping to the target
/// inside [`SHELL_SNAP`].
///
/// Pure, so the growth rule and the half-life are testable apart from the ECS.
pub(crate) fn eased_shell_envelope(current: f32, target: f32, dt: f32) -> f32 {
    if target >= current {
        return target;
    }
    // alpha = 1 - 2^(-dt / half_life): the fraction of the remaining distance
    // this frame covers, which is what makes the half-life the same wall-clock
    // time at any frame rate.
    let alpha = 1.0 - (-dt / SHRINK_HALF_LIFE_SECS).exp2();
    let next = current + (target - current) * alpha;
    if (next - target).abs() <= SHELL_SNAP.to_engine() {
        target
    } else {
        next
    }
}

/// Sizes the directional-HUD shells off their hulls.
///
/// Inits [`HudShellEnvelopes`], registers the shell types, adds the
/// hide-until-measured observer, and runs `ease_shell_envelopes` and
/// `sync_shell_radii` in Update within [`super::NovaHudSystems`], after the
/// widget's own input pass so a shell's visibility rule is the last word.
#[derive(Default)]
pub struct HullShellPlugin;

impl Plugin for HullShellPlugin {
    fn build(&self, app: &mut App) {
        trace!("HullShellPlugin: build");

        app.init_resource::<HudShellEnvelopes>();
        app.register_type::<HullShellClearance>();
        app.register_type::<HullShellPending>();

        app.add_observer(hide_shell_until_measured);

        app.add_systems(
            Update,
            (ease_shell_envelopes, sync_shell_radii)
                .chain()
                .after(super::velocity::update_velocity_hud_input)
                .in_set(super::NovaHudSystems),
        );
    }
}

/// A shell starts dark. The widget bundle asks for a visible sphere; this takes
/// it straight back out, so no frame can carry a shell at a radius its hull has
/// not published yet.
fn hide_shell_until_measured(add: On<Add, HullShellPending>, mut q_shell: Query<&mut Visibility>) {
    if let Ok(mut visibility) = q_shell.get_mut(add.entity) {
        visibility.set_if_neq(Visibility::Hidden);
    }
}

/// Track each watched hull's envelope, eased.
pub(crate) fn ease_shell_envelopes(
    time: Res<Time>,
    mut watched: Local<EntityHashSet>,
    mut envelopes: ResMut<HudShellEnvelopes>,
    q_shell: Query<&VelocityHudTargetEntity, With<HullShellClearance>>,
    q_hull: Query<&HullEnvelopeRadius>,
) {
    watched.clear();
    for target in &q_shell {
        watched.insert(**target);
    }
    // A hull nobody draws a shell around any more is not a hull to keep easing.
    envelopes.retain(|hull, _| watched.contains(hull));

    let dt = time.delta_secs();
    for &hull in watched.iter() {
        // A hull that has stopped publishing (a wreck: no live sections left)
        // keeps its last radius rather than collapsing the shell through the
        // debris still on screen.
        let Ok(target) = q_hull.get(hull).map(|radius| **radius) else {
            continue;
        };
        match envelopes.get(&hull) {
            // The first reading is the shell's size outright: there is nothing
            // to ease from.
            None => envelopes.insert(hull, target),
            Some(&current) => envelopes.insert(hull, eased_shell_envelope(current, target, dt)),
        };
    }
}

/// Drive every consumer of a shell's radius off the one eased number: the orbit
/// the cone rides, the sphere child's offset and scale, and the shader's own
/// radius.
pub(crate) fn sync_shell_radii(
    mut commands: Commands,
    envelopes: Res<HudShellEnvelopes>,
    mut q_shell: Query<
        (
            Entity,
            &HullShellClearance,
            &VelocityHudTargetEntity,
            &VelocityHudSource,
            &mut DirectionalSphereOrbit,
            &mut Visibility,
            Option<&Children>,
            Has<HullShellPending>,
        ),
        With<VelocityHudMarker>,
    >,
    mut q_sphere: Query<
        (
            &mut Transform,
            &MeshMaterial3d<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>,
        ),
        With<VelocityHudSphereMarker>,
    >,
    mut sphere_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>,
    >,
) {
    for (entity, clearance, target, source, mut orbit, mut visibility, children, pending) in
        &mut q_shell
    {
        let Some(&envelope) = envelopes.get(&**target) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let (velocity_radius, gravity_radius) = shell_radii(envelope);
        // Both shells are derived from the same envelope every frame, so the
        // authored separation between them is exact at every radius, including
        // mid-convergence.
        let radius = match source {
            VelocityHudSource::Velocity => velocity_radius,
            VelocityHudSource::Gravity => gravity_radius,
        };
        // Belt: a clearance that disagreed with the pair above would be a
        // widget sized outside this contract, and the shells would drift.
        debug_assert!(
            (radius - (envelope + clearance.to_engine())).abs() < 1e-3,
            "shell clearance {:?} does not match its source {source:?}",
            **clearance
        );

        let resized = (orbit.radius - radius).abs() > f32::EPSILON;
        if resized {
            orbit.radius = radius;
            for &child in children.into_iter().flatten() {
                let Ok((mut transform, material)) = q_sphere.get_mut(child) else {
                    continue;
                };
                // The widget root sits a radius out along the direction and the
                // sphere sits a radius back down it, so the sphere is centred
                // on the hull's own centre of mass.
                *transform = Transform::from_translation(Vec3::new(0.0, 0.0, radius))
                    .with_scale(Vec3::splat(radius));
                if let Some(mut material) = sphere_materials.get_mut(&**material) {
                    material.extension.radius = radius;
                }
            }
        }

        if pending {
            commands.entity(entity).remove::<HullShellPending>();
            // The velocity readout is unconditional, so measuring it is what
            // reveals it. The gravity shell keeps its own rule - it is up only
            // in a well - and its driver already wrote this frame's answer.
            if *source == VelocityHudSource::Velocity {
                visibility.set_if_neq(Visibility::Visible);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// The contract in meters: 5 m outside the hull, 6 m between the shells,
    /// whatever the hull's size. Read back through the meter types rather than
    /// re-running the world-unit arithmetic under test.
    #[test]
    fn the_shells_stand_at_their_authored_metric_clearances() {
        for envelope_m in [12.5, 195.0] {
            let envelope = Meters(envelope_m).to_engine();
            let (velocity, gravity) = shell_radii(envelope);

            let hull_gap = Meters::from_engine(velocity - envelope);
            let shell_gap = Meters::from_engine(gravity - velocity);
            assert!(
                (hull_gap.get() - 5.0).abs() < 1e-3,
                "{envelope_m} m hull: velocity shell {hull_gap:?} outside the envelope"
            );
            assert!(
                (shell_gap.get() - 6.0).abs() < 1e-3,
                "{envelope_m} m hull: gravity shell {shell_gap:?} outside the velocity shell"
            );
            assert!(gravity > velocity, "the gravity shell is the outer one");
        }
    }

    /// A GROWING hull is covered on the frame it grows: an eased expansion
    /// would leave the hull outside its own shell for as long as it lasted.
    #[test]
    fn growth_is_immediate() {
        assert_eq!(eased_shell_envelope(1.0, 20.0, 1.0 / 60.0), 20.0);
        assert_eq!(eased_shell_envelope(1.0, 1.0, 1.0 / 60.0), 1.0);
    }

    /// Shrinking covers exactly half the distance in 150 ms, at any frame rate:
    /// the same wall-clock half-life whether it is reached in one step or in
    /// sixty.
    #[test]
    fn shrinking_halves_the_gap_every_150ms() {
        let start = 20.0;
        let target = 10.0;

        let one_step = eased_shell_envelope(start, target, 0.150);
        assert!((one_step - 15.0).abs() < 1e-4, "one step: {one_step}");

        let mut stepped = start;
        for _ in 0..9 {
            stepped = eased_shell_envelope(stepped, target, 0.150 / 9.0);
        }
        assert!(
            (stepped - one_step).abs() < 1e-4,
            "nine steps must land where one did: {stepped} vs {one_step}"
        );
    }

    /// The approach ENDS. An exponential never arrives, so inside a centimetre
    /// the radius is simply the target - which is also what stops a shell
    /// re-uploading its material forever.
    #[test]
    fn the_last_centimetre_snaps_to_the_target() {
        let target = 10.0;
        // 9 mm out, in meters: inside the snap.
        let near = target + Meters(0.009).to_engine();
        assert_eq!(eased_shell_envelope(near, target, 1.0 / 600.0), target);

        // 50 cm out: still easing.
        let far = target + Meters(0.5).to_engine();
        assert!(eased_shell_envelope(far, target, 1.0 / 600.0) > target);
    }

    /// A hull whose envelope arrives takes it outright, and a later SHRINK is
    /// the only thing that eases.
    #[test]
    fn the_first_reading_is_taken_outright_then_shrinks_ease() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<HudShellEnvelopes>();
        let hull = world.spawn(HullEnvelopeRadius(20.0)).id();
        world.spawn((
            HullShellClearance(HULL_CLEARANCE),
            VelocityHudTargetEntity::new(hull),
        ));

        world.run_system_once(ease_shell_envelopes).unwrap();
        assert_eq!(
            world.resource::<HudShellEnvelopes>().get(&hull).copied(),
            Some(20.0),
            "the first reading is the shell's size, not something to ease to"
        );

        // Half the hull is shot away; with no time elapsed the shell has not
        // moved yet.
        world.entity_mut(hull).insert(HullEnvelopeRadius(10.0));
        world.run_system_once(ease_shell_envelopes).unwrap();
        let held = world.resource::<HudShellEnvelopes>()[&hull];
        assert!(held > 10.0, "a shrink eases rather than stepping: {held}");
    }

    /// A hull nobody watches stops being tracked, so the map cannot grow with
    /// every ship that ever carried a shell.
    #[test]
    fn an_unwatched_hull_is_dropped() {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<HudShellEnvelopes>();
        let hull = world.spawn(HullEnvelopeRadius(20.0)).id();
        let shell = world
            .spawn((
                HullShellClearance(HULL_CLEARANCE),
                VelocityHudTargetEntity::new(hull),
            ))
            .id();
        world.run_system_once(ease_shell_envelopes).unwrap();
        assert!(world.resource::<HudShellEnvelopes>().contains_key(&hull));

        world.despawn(shell);
        world.run_system_once(ease_shell_envelopes).unwrap();
        assert!(world.resource::<HudShellEnvelopes>().is_empty());
    }

    /// A shell widget with the one child the sizing pass drives, and a live
    /// material asset behind it.
    fn spawn_shell(
        world: &mut World,
        hull: Entity,
        source: VelocityHudSource,
        clearance: Meters,
    ) -> (
        Entity,
        Entity,
        Handle<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>,
    ) {
        let material = world
            .resource_mut::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>()
            .add(ExtendedMaterial {
                base: StandardMaterial::default(),
                extension: DirectionSphereMaterial::default(),
            });
        let shell = world
            .spawn((
                VelocityHudMarker,
                HullShellClearance(clearance),
                HullShellPending,
                VelocityHudTargetEntity::new(hull),
                source,
                DirectionalSphereOrbit {
                    radius: 0.0,
                    center: Vec3::ZERO,
                    direction: Vec3::NEG_Z,
                    smoothing: 0.0,
                },
                Visibility::Hidden,
            ))
            .id();
        let sphere = world
            .spawn((
                VelocityHudSphereMarker,
                ChildOf(shell),
                Transform::default(),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        (shell, sphere, material)
    }

    fn shell_world() -> World {
        let mut world = World::new();
        world.init_resource::<HudShellEnvelopes>();
        world
            .init_resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>();
        world
    }

    /// Every consumer of the radius is driven off the ONE eased number: the
    /// orbit the cone rides, the sphere child's offset and scale, and the
    /// shader's own radius. A child left behind draws a sphere of one size in a
    /// shell of another.
    #[test]
    fn every_consumer_of_the_radius_is_synced_from_the_eased_envelope() {
        let mut world = shell_world();
        let hull = world.spawn_empty().id();
        world.resource_mut::<HudShellEnvelopes>().insert(hull, 18.0);
        let (shell, sphere, material) = spawn_shell(
            &mut world,
            hull,
            VelocityHudSource::Velocity,
            HULL_CLEARANCE,
        );

        world.run_system_once(sync_shell_radii).unwrap();
        world.flush();

        let radius = world
            .entity(shell)
            .get::<DirectionalSphereOrbit>()
            .unwrap()
            .radius;
        assert!(radius > 18.0, "the shell stands outside the hull: {radius}");
        // The sphere sits a radius back down the widget's own axis and is
        // scaled to it, so it is centred on the hull at exactly this size.
        let transform = *world.entity(sphere).get::<Transform>().unwrap();
        assert_eq!(transform.translation, Vec3::new(0.0, 0.0, radius));
        assert_eq!(transform.scale, Vec3::splat(radius));
        let shader_radius = world
            .resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>()
            .get(&material)
            .unwrap()
            .extension
            .radius;
        assert_eq!(shader_radius, radius);

        // The hull loses half of itself: every consumer follows the new number
        // together.
        world.resource_mut::<HudShellEnvelopes>().insert(hull, 9.0);
        world.run_system_once(sync_shell_radii).unwrap();
        let shrunk = world
            .entity(shell)
            .get::<DirectionalSphereOrbit>()
            .unwrap()
            .radius;
        assert!(shrunk < radius, "{shrunk} vs {radius}");
        let transform = *world.entity(sphere).get::<Transform>().unwrap();
        assert_eq!(transform.translation, Vec3::new(0.0, 0.0, shrunk));
        assert_eq!(transform.scale, Vec3::splat(shrunk));
        let shader_radius = world
            .resource::<Assets<ExtendedMaterial<StandardMaterial, DirectionSphereMaterial>>>()
            .get(&material)
            .unwrap()
            .extension
            .radius;
        assert_eq!(shader_radius, shrunk);
    }

    /// A shell stays dark until its hull has been measured. Flashing a sphere
    /// at a guessed radius while a carrier is still assembling is exactly the
    /// wrong first frame.
    #[test]
    fn a_shell_is_hidden_until_its_hull_is_measured() {
        let mut world = shell_world();
        let hull = world.spawn_empty().id();
        let (shell, ..) = spawn_shell(
            &mut world,
            hull,
            VelocityHudSource::Velocity,
            HULL_CLEARANCE,
        );

        world.run_system_once(sync_shell_radii).unwrap();
        world.flush();
        assert_eq!(
            *world.entity(shell).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
        assert!(world.entity(shell).contains::<HullShellPending>());

        world.resource_mut::<HudShellEnvelopes>().insert(hull, 18.0);
        world.run_system_once(sync_shell_radii).unwrap();
        world.flush();
        assert_eq!(
            *world.entity(shell).get::<Visibility>().unwrap(),
            Visibility::Visible,
            "measuring the hull is what reveals the velocity shell"
        );
        assert!(!world.entity(shell).contains::<HullShellPending>());
    }

    /// The gravity shell keeps its OWN visibility rule: it is up only in a
    /// well, and being measured does not overrule the driver that hid it in
    /// flat space.
    #[test]
    fn measuring_the_hull_does_not_reveal_the_gravity_shell() {
        let mut world = shell_world();
        let hull = world.spawn_empty().id();
        world.resource_mut::<HudShellEnvelopes>().insert(hull, 18.0);
        let clearance = Meters(HULL_CLEARANCE.get() + SHELL_SEPARATION.get());
        let (shell, ..) = spawn_shell(&mut world, hull, VelocityHudSource::Gravity, clearance);

        world.run_system_once(sync_shell_radii).unwrap();
        world.flush();

        assert_eq!(
            *world.entity(shell).get::<Visibility>().unwrap(),
            Visibility::Hidden,
            "flat space keeps the gravity shell down"
        );
        assert!(!world.entity(shell).contains::<HullShellPending>());
        // It is still SIZED, so the frame it is shown it is already right.
        let radius = world
            .entity(shell)
            .get::<DirectionalSphereOrbit>()
            .unwrap()
            .radius;
        assert!(radius > 18.0, "got {radius}");
    }
}
