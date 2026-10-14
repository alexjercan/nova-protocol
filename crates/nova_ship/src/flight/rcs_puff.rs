//! The RCS puff: what a reaction-control push LOOKS like.
//!
//! The RCS primitive (`rcs_burn_system`) is one torque-free impulse at the
//! centre of mass, and the fiction is cold-gas quads on the hull. With no
//! physical thruster to light, a push would show nothing, and an escort sliding
//! on its RCS reads as a ship dragged by an invisible hand. So every ship that
//! delivers an RCS push throws small gas puffs off the hull face
//! OPPOSITE the push - the side a cold-gas jet would have to fire from - moving
//! away from the hull.
//!
//! Driven by [`RcsBudget::applied`], the thrust the burn actually delivered,
//! like the RCS hiss: a refused or starved command puffs nothing. The player's
//! ship and every AI ship take the same path.
//!
//! A puff is a mesh entity with no rigid body, so the save classifies it as a
//! visual and the broad phase never sees it. Engine units throughout: one unit
//! is 10 m.

use avian3d::prelude::{ComputedCenterOfMass, LinearVelocity};
use bevy::prelude::*;
use nova_gameplay::prelude::{Fnv32, SectionInactiveMarker, SectionMarker, SpaceshipRootMarker};

use super::state::{RcsBudget, RcsIntent};

/// `RcsPuffMarker` and `RcsPuffPlugin`.
pub mod prelude {
    pub use super::{RcsPuffMarker, RcsPuffPlugin};
}

/// Seconds between puffs while a ship holds any delivered push.
const PUFF_INTERVAL_SECS: f32 = 0.06;

/// How long one puff lives. Short: it is a jet of gas, not a cloud that hangs
/// over the ship.
const PUFF_LIFETIME_SECS: f32 = 0.6;

/// A puff's widest diameter at full deflection, world units (15 m, one and a
/// half cells). Sized on an escort 300 m off: at one cell the puffs read as
/// grey studs on the hull. A lighter push throws a smaller one, down to
/// [`PUFF_MIN_SHARE`] of this.
const PUFF_PEAK_SIZE: f32 = 1.5;

/// The share of [`PUFF_PEAK_SIZE`] the lightest delivered push puffs at, so a
/// station-keeping trim is still seen.
const PUFF_MIN_SHARE: f32 = 0.5;

/// How fast a puff leaves the hull, world units per second, on top of the
/// ship's own velocity.
const PUFF_SPEED: f32 = 3.0;

/// Sections whose reach along the jet is within this of the outermost one share
/// the face a puff can come off, world units. Spreads the puffs along a long
/// flank instead of stacking them on one section.
const PUFF_FACE_BAND: f32 = 0.6;

/// Puff colour: cold gas, pale blue, over 1.0 so it blooms against space.
/// Drawn unlit and ADDED to what is behind it, so overlapping puffs build into
/// a jet and a puff over the hull brightens it rather than hiding it. The
/// alpha scales the add.
const PUFF_COLOR: LinearRgba = LinearRgba::new(2.0, 2.3, 2.8, 0.6);

/// A puff in flight off a ship's RCS. A range or an example counts these to
/// assert a push was shown.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct RcsPuffMarker;

/// One puff's motion and its age.
#[derive(Component, Clone, Copy, Debug)]
struct RcsPuff {
    /// World velocity, units per second: the ship's velocity at release plus
    /// the jet.
    velocity: Vec3,
    /// Widest diameter this puff reaches, world units.
    size: f32,
    /// Seconds since release.
    age: f32,
}

/// A ship root's puff clock: seconds to the next puff, and how many it has
/// thrown, which picks the section the next one comes off.
#[derive(Component, Clone, Copy, Debug, Default)]
struct RcsPuffClock {
    next: f32,
    thrown: u32,
}

/// The one mesh and material every puff is drawn with, built once: a firing
/// ship throws one every [`PUFF_INTERVAL_SECS`].
#[derive(Resource, Debug)]
struct RcsPuffAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for RcsPuffAssets {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(
            Sphere::new(0.5)
                .mesh()
                .ico(1)
                .expect("ico subdivision 1 is valid"),
        );
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::LinearRgba(PUFF_COLOR),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                ..default()
            });
        Self { mesh, material }
    }
}

/// Throws RCS puffs off every ship delivering an RCS push. Render-only: a
/// headless run has no use for them.
pub struct RcsPuffPlugin;

impl Plugin for RcsPuffPlugin {
    fn build(&self, app: &mut App) {
        trace!("RcsPuffPlugin: build");

        app.register_type::<RcsPuffMarker>();
        app.init_resource::<RcsPuffAssets>();
        app.add_systems(Update, (throw_rcs_puffs, drift_rcs_puffs));
    }
}

/// A puff's diameter at `t` of its life (`0..1`), as a share of its peak:
/// released at a third, swelling as the gas spreads, gone at the end.
fn puff_scale(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (0.35 + 0.65 * t.sqrt()) * (1.0 - t * t)
}

/// The world point a puff comes off: a live section on the hull face the jet
/// points out of, picked by `seed` among the sections within
/// [`PUFF_FACE_BAND`] of the outermost one. `None` for a hull with no live
/// section.
fn puff_origin(
    sections: impl Iterator<Item = Vec3>,
    middle: Vec3,
    jet: Vec3,
    seed: u32,
) -> Option<Vec3> {
    let points: Vec<(f32, Vec3)> = sections
        .map(|point| ((point - middle).dot(jet), point))
        .collect();
    let reach = points
        .iter()
        .map(|(reach, _)| *reach)
        .fold(f32::NEG_INFINITY, f32::max);
    let face: Vec<Vec3> = points
        .into_iter()
        .filter(|(along, _)| *along >= reach - PUFF_FACE_BAND)
        .map(|(_, point)| point)
        .collect();
    if face.is_empty() {
        return None;
    }
    Some(face[seed as usize % face.len()])
}

#[expect(
    clippy::type_complexity,
    reason = "one query over the ship root's RCS state, pose and motion"
)]
fn throw_rcs_puffs(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<RcsPuffAssets>,
    mut q_ships: Query<
        (
            Entity,
            &RcsBudget,
            &RcsIntent,
            &GlobalTransform,
            Option<&ComputedCenterOfMass>,
            Option<&LinearVelocity>,
            Option<&Children>,
            Option<&mut RcsPuffClock>,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_sections: Query<&GlobalTransform, (With<SectionMarker>, Without<SectionInactiveMarker>)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        // A paused clock: the last tick's push is still on the budget, and
        // throwing on it would fill a frozen frame with puffs.
        return;
    }
    for (root, budget, intent, pose, center_of_mass, velocity, children, clock) in &mut q_ships {
        let push = intent.0.normalize_or_zero();
        if budget.applied <= 1e-4 || push == Vec3::ZERO {
            if let Some(mut clock) = clock {
                clock.next = 0.0;
            }
            continue;
        }
        let mut state = clock.as_deref().copied().unwrap_or_default();
        state.next -= dt;
        if state.next > 0.0 {
            if let Some(mut clock) = clock {
                *clock = state;
            }
            continue;
        }
        state.next = PUFF_INTERVAL_SECS;
        state.thrown = state.thrown.wrapping_add(1);
        match clock {
            Some(mut clock) => *clock = state,
            None => {
                commands.entity(root).try_insert(state);
            }
        }

        // The gas leaves the hull against the push.
        let jet = -(pose.rotation() * push);
        let middle = pose.transform_point(center_of_mass.map_or(Vec3::ZERO, |com| com.0));
        let seed = Fnv32::new()
            .write(&root.to_bits().to_le_bytes())
            .write(&state.thrown.to_le_bytes())
            .finish();
        let sections = children
            .into_iter()
            .flatten()
            .filter_map(|child| q_sections.get(*child).ok())
            .map(GlobalTransform::translation);
        let Some(origin) = puff_origin(sections, middle, jet, seed) else {
            continue;
        };
        let effort = budget.applied.min(1.0);
        let size = PUFF_PEAK_SIZE * (PUFF_MIN_SHARE + (1.0 - PUFF_MIN_SHARE) * effort);
        let ship_velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
        commands.spawn((
            Name::new("RCS Puff"),
            RcsPuffMarker,
            RcsPuff {
                velocity: ship_velocity + jet * PUFF_SPEED,
                size,
                age: 0.0,
            },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            // Half a cell out, so the puff starts on the section's outer face
            // rather than inside it.
            Transform::from_translation(origin + jet * 0.5)
                .with_scale(Vec3::splat(size * puff_scale(0.0))),
        ));
    }
}

fn drift_rcs_puffs(
    mut commands: Commands,
    time: Res<Time>,
    mut q_puffs: Query<(Entity, &mut RcsPuff, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut puff, mut transform) in &mut q_puffs {
        puff.age += dt;
        if puff.age >= PUFF_LIFETIME_SECS {
            commands.entity(entity).try_despawn();
            continue;
        }
        transform.translation += puff.velocity * dt;
        transform.scale = Vec3::splat(puff.size * puff_scale(puff.age / PUFF_LIFETIME_SECS));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame of the test clock.
    const TEST_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

    fn puff_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(TEST_FRAME));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(RcsPuffPlugin);
        // A manual clock's first tick is dt 0, and a paused clock throws
        // nothing.
        app.update();
        app
    }

    /// A three-section hull along X at `at`, pushing `push` (ship-local) and
    /// delivering `applied` of it.
    fn spawn_pushing_ship(app: &mut App, at: Vec3, push: Vec3, applied: f32) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::from_translation(at),
                ComputedCenterOfMass(Vec3::ZERO),
                LinearVelocity(Vec3::ZERO),
                RcsIntent(push),
                RcsBudget {
                    applied,
                    ..default()
                },
            ))
            .id();
        for x in [-1.0, 0.0, 1.0] {
            app.world_mut().spawn((
                SectionMarker,
                GlobalTransform::from_translation(at + Vec3::X * x),
                ChildOf(root),
            ));
        }
        root
    }

    fn puffs(app: &mut App) -> Vec<(Vec3, Vec3)> {
        app.world_mut()
            .query_filtered::<(&Transform, &RcsPuff), With<RcsPuffMarker>>()
            .iter(app.world())
            .map(|(transform, puff)| (transform.translation, puff.velocity))
            .collect()
    }

    /// An AI ship 300 m off, pushing to starboard: the gas comes off its port
    /// face and moves away to port, which is the side a cold-gas jet pushing
    /// it to starboard fires from.
    #[test]
    fn a_ship_pushing_on_its_rcs_puffs_gas_off_the_face_opposite_the_push() {
        let mut app = puff_app();
        let at = Vec3::new(0.0, 0.0, -30.0);
        spawn_pushing_ship(&mut app, at, Vec3::X, 0.25);
        for _ in 0..10 {
            app.update();
        }

        let puffs = puffs(&mut app);
        assert!(!puffs.is_empty(), "a delivered push throws puffs");
        for (position, velocity) in puffs {
            assert!(
                position.x < at.x - 1.0,
                "a puff sits off the port face, got {position}"
            );
            assert!(velocity.x < 0.0, "and leaves to port, got {velocity}");
        }
    }

    /// A refused or starved command pushed nothing, so it shows nothing.
    #[test]
    fn a_push_that_delivers_nothing_puffs_nothing() {
        let mut app = puff_app();
        spawn_pushing_ship(&mut app, Vec3::ZERO, Vec3::X, 0.0);
        for _ in 0..10 {
            app.update();
        }
        assert!(puffs(&mut app).is_empty());
    }

    /// Puffs are gas, not debris: they clear themselves once the push stops.
    #[test]
    fn every_puff_clears_itself_after_the_push_stops() {
        let mut app = puff_app();
        let ship = spawn_pushing_ship(&mut app, Vec3::ZERO, Vec3::X, 1.0);
        for _ in 0..5 {
            app.update();
        }
        assert!(!puffs(&mut app).is_empty(), "delivery guard: it puffed");

        app.world_mut().get_mut::<RcsBudget>(ship).unwrap().applied = 0.0;
        let frames = (PUFF_LIFETIME_SECS / TEST_FRAME.as_secs_f32()).ceil() as usize + 2;
        for _ in 0..frames {
            app.update();
        }
        assert!(puffs(&mut app).is_empty(), "every puff is gone");
    }
}
