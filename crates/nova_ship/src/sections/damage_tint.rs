//! SCORCH: what damage does to a section's colour.
//!
//! One damage effect among several, and the oldest of them. It reads
//! [`DamageLevel`] - the share of a section's own health that is gone - and
//! grades that section's material by it: an untouched section keeps its
//! authored look, a battered one reddens and darkens, a critical one glows, and
//! a dead one reads burnt.
//!
//! # Every ship, not just the player's
//!
//! Grading used to be split by the ship root's [`Allegiance`]: the player ship
//! got the full gradient, an enemy stayed pristine until it died, and neutral
//! bodies were never graded at all. That split is gone.
//!
//! It cannot survive geometry. Erosion answers to the same level this does, so
//! a hull that visibly wears through cannot be wearing paint that claims the
//! section is fine - the ship would contradict itself. Showing damage on
//! everything is the better read anyway: what the player has already chewed
//! through is legible at a glance rather than only at the moment it dies.
//!
//! Anything hanging off a [`SectionFixture`] is still skipped - see
//! `owning_section`. A plate has its own health and its own effect (it wears
//! down), and grading it here would redden the paint of a ship whose structure
//! has not been touched.
//!
//! ## Why per-section material clones
//!
//! Sections render via gltf `WorldAssetRoot` scenes (see
//! `assets/base/sections/base.content.ron`), and a gltf scene's materials are
//! shared `Handle<StandardMaterial>`s across every instance of the same mesh -
//! so mutating a material in place would tint every section that shares that
//! mesh at once. Grading is per-section, so each rendered mesh gets a private
//! clone of its material (captured with its pristine look) that this module
//! owns and mutates. The cuboid fallback path already hands each section a
//! unique `materials.add(...)` handle, but cloning uniformly keeps one code
//! path and is harmless.
//!
//! ## Timing
//!
//! Capture keys on `Added<MeshMaterial3d<StandardMaterial>>`, which fires the
//! frame a mesh appears - whether it is a synchronous cuboid or an
//! asynchronously instantiated gltf node - so it does not depend on any
//! scene-ready signal. The ship root's marker (and its required `Allegiance`)
//! is inserted synchronously when the ship spawns
//! (`nova_scenario::objects::spaceship`), long before async gltf materials
//! load, so reading the root's allegiance at capture time is safe.

use bevy::prelude::*;
#[cfg(test)]
use nova_gameplay::prelude::{Allegiance, DamageLevelPlugin, Health, SectionInactiveMarker};
use nova_gameplay::prelude::{DamageLevel, SectionMarker};

#[cfg(test)]
use crate::sections::damage_effects::prelude::{DamageEffects, DamageEffectsPlugin};
use crate::sections::fixture::prelude::SectionFixture;

/// `DamageScorch`, `SectionDamageTint` and `SectionDamageTintPlugin`.
pub mod prelude {
    pub use super::{DamageScorch, SectionDamageTint, SectionDamageTintPlugin};
}

/// Makes a section's paint redden, darken and finally burn out as its damage
/// level rises.
///
/// Carried by the SECTION rather than by the meshes drawing it: one section
/// owns one health pool and its whole rendered body grades together. Fitted
/// from the authored
/// [`DamageEffects`](super::damage_effects::DamageEffects), whose default
/// carries it - so a section that never mentions effects scorches exactly as
/// every section did before this was authorable.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DamageScorch;

/// Below this integrity ratio the section starts to visibly redden/darken.
const WARN_BELOW: f32 = 0.85;
/// At or below this integrity ratio the section glows red (critical).
const GLOW_BELOW: f32 = 0.4;
/// How far a fully-damaged (but not dead) section tints toward `DAMAGE_RED`.
const MAX_REDDEN: f32 = 0.9;
/// How far a fully-damaged (but not dead) section darkens toward black. The
/// brightness drop is the colour-blind-safe half of the cue.
const MAX_DARKEN: f32 = 0.45;
/// The reddening target colour.
const DAMAGE_RED: Color = Color::srgb(0.55, 0.05, 0.03);
/// The burnt look of a dead or disabled section.
const DEAD_COLOR: Color = Color::srgb(0.05, 0.02, 0.02);
/// The integrity ratio below which a section burns out: the glow fades and the
/// colour goes to [`DEAD_COLOR`], so the very end of a section's life reads as
/// cold wreckage rather than as something still hot.
const BURNT_BELOW: f32 = 0.1;
/// Peak red emissive glow added at zero integrity (before death).
const GLOW_PEAK: LinearRgba = LinearRgba::new(2.2, 0.18, 0.05, 1.0);

/// Records a rendered mesh whose material is graded by a section's health.
///
/// Holds the private (cloned) material handle this module owns and mutates, plus
/// the pristine look captured before any grading, so a section that heals or a
/// design that later removes the tint can restore the original appearance and so
/// grading never bleeds across sections that shared a gltf material.
#[derive(Component, Clone, Debug)]
pub struct SectionDamageTint {
    /// The section entity whose `Health` drives this mesh's tint.
    pub section: Entity,
    /// The private material this component owns and writes to each frame.
    pub material: Handle<StandardMaterial>,
    /// The pristine base colour, captured before grading.
    pub base_color: Color,
    /// The pristine emissive, captured before grading.
    pub emissive: LinearRgba,
}

/// Grades player-ship section materials by integrity so the ship shows its own
/// health. Registered by the section plugin only when rendering is enabled.
#[derive(Default, Clone, Debug)]
pub struct SectionDamageTintPlugin;

impl Plugin for SectionDamageTintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                mark_section_meshes,
                resolve_pending_tints,
                grade_section_tints,
            )
                .chain(),
        );
    }
}

/// A player-ship section mesh awaiting material capture. Its `StandardMaterial`
/// handle may exist before the asset itself resolves (async gltf load), so
/// marking is decoupled from capture: `mark_section_meshes` tags the mesh once
/// (doing the ChildOf walk), and `resolve_pending_tints` retries the clone every
/// frame until the asset is available - self-re-arming, so a not-yet-loaded
/// material can never silently drop the mesh out of grading (review R1.1).
#[derive(Component, Clone, Copy, Debug)]
struct PendingSectionTint {
    section: Entity,
}

/// Walk up the `ChildOf` chain from `entity` to the nearest ancestor that is a
/// section WEARING SCORCH, returning that section entity. Returns `None` if the
/// walk reaches a [`SectionFixture`], or leaves the tree without passing
/// through such a section.
///
/// A section that does not wear scorch ends the walk the same way a fixture
/// does. It must not fall through to the section ABOVE it: grading a turret's
/// meshes by the hull it is bolted to would report the wrong pool, which is
/// worse than reporting nothing.
fn owning_section(
    entity: Entity,
    q_child_of: &Query<&ChildOf>,
    q_is_section: &Query<(), With<SectionMarker>>,
    q_is_fixture: &Query<(), With<SectionFixture>>,
    q_scorches: &Query<(), With<DamageScorch>>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        // A fixture ENDS the walk, and is not graded by what it hangs on. Its
        // damage read is that it comes off, so it must not redden on the way
        // there - and grading it by the section behind it would be worse than
        // nothing: fresh cladding over a dying hull would read burnt.
        if q_is_fixture.get(current).is_ok() {
            return None;
        }
        if q_is_section.get(current).is_ok() {
            return q_scorches.get(current).is_ok().then_some(current);
        }
        current = q_child_of.get(current).ok()?.0;
    }
}

/// Tag every freshly-spawned ship section mesh for tint capture, recording the
/// grading mode its owning ship's allegiance selects. The ChildOf walk and the
/// allegiance gate happen here, once per mesh; the actual material clone is
/// deferred to `resolve_pending_tints` so a not-yet-loaded asset does not drop
/// the mesh.
#[expect(
    clippy::type_complexity,
    reason = "the Added filter carries its own Without guards"
)]
fn mark_section_meshes(
    mut commands: Commands,
    q_new: Query<
        Entity,
        (
            Added<MeshMaterial3d<StandardMaterial>>,
            Without<SectionDamageTint>,
            Without<PendingSectionTint>,
        ),
    >,
    q_child_of: Query<&ChildOf>,
    q_is_section: Query<(), With<SectionMarker>>,
    q_is_fixture: Query<(), With<SectionFixture>>,
    q_scorches: Query<(), With<DamageScorch>>,
) {
    for entity in &q_new {
        let Some(section) = owning_section(
            entity,
            &q_child_of,
            &q_is_section,
            &q_is_fixture,
            &q_scorches,
        ) else {
            continue;
        };

        // No allegiance test any more: every ship shows what it has taken, so
        // there is nothing to look up on the root and nothing to exclude.
        //
        // `try_insert`, not `insert`: a section mesh can be chain-destroyed the
        // same frame it gains its material (a ship exploding), despawning this
        // entity before the buffer applies - the insert must be a no-op there,
        // not a panic.
        commands
            .entity(entity)
            .try_insert(PendingSectionTint { section });
    }
}

/// Clone the material of each pending section mesh into a private handle once its
/// asset is available, so `grade_section_tints` can tint each section
/// independently without mutating the shared source material. Retries until the
/// asset loads; this query is normally empty.
fn resolve_pending_tints(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_pending: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &PendingSectionTint,
    )>,
) {
    for (entity, material, pending) in &q_pending {
        let Some(pristine) = materials.get(&material.0).cloned() else {
            // Asset not loaded yet; keep the pending marker and retry next frame.
            continue;
        };

        let base_color = pristine.base_color;
        let emissive = pristine.emissive;
        let handle = materials.add(pristine);

        // NOTE: same despawn race as `mark_section_meshes` - a pending section
        // mesh can be chain-destroyed before this resolves, so tolerate a
        // missing entity rather than panic.
        commands
            .entity(entity)
            .try_insert((
                MeshMaterial3d(handle.clone()),
                SectionDamageTint {
                    section: pending.section,
                    material: handle,
                    base_color,
                    emissive,
                },
            ))
            .try_remove::<PendingSectionTint>();
    }
}

/// Grade every captured section mesh by its section's current integrity. The
/// target look is written only when it differs from the material's current
/// value, so an idle (undamaged, unchanging) ship does not re-flag its materials
/// as changed every frame (review R1.2).
fn grade_section_tints(
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_tints: Query<&SectionDamageTint>,
    q_level: Query<&DamageLevel, With<SectionMarker>>,
) {
    for tint in &q_tints {
        let (base_color, emissive) = match q_level.get(tint.section) {
            // The SAME level erosion reads, so paint and geometry cannot
            // disagree about how far gone a section is.
            Ok(level) => damage_look(1.0 - level.0, tint.base_color, tint.emissive),
            // Section gone (mid-despawn) or never had health: leave pristine.
            Err(_) => (tint.base_color, tint.emissive),
        };

        // Read first; only take a mutable (change-flagging) borrow on a real
        // change.
        let Some(current) = materials.get(&tint.material) else {
            continue;
        };
        if current.base_color == base_color && current.emissive == emissive {
            continue;
        }

        let Some(mut material) = materials.get_mut(&tint.material) else {
            continue;
        };
        material.base_color = base_color;
        material.emissive = emissive;
    }
}

/// The graded look for a living section at integrity `ratio` (1.0 healthy, 0.0
/// dead), given its pristine colours. Redden + darken from `WARN_BELOW` down, and
/// add a rising red glow from `GLOW_BELOW` down.
fn damage_look(ratio: f32, base_color: Color, base_emissive: LinearRgba) -> (Color, LinearRgba) {
    // 0.0 at/above WARN_BELOW, 1.0 at zero integrity.
    let hurt = ((WARN_BELOW - ratio) / WARN_BELOW).clamp(0.0, 1.0);
    let reddened = base_color.mix(&DAMAGE_RED, hurt * MAX_REDDEN);
    let base = reddened.mix(&Color::BLACK, hurt * MAX_DARKEN);

    let glow_t = ((GLOW_BELOW - ratio) / GLOW_BELOW).clamp(0.0, 1.0);

    // The burnt endpoint, and it belongs to EVERY ship now rather than only to
    // the ones the player was not flying. The last of a section's integrity
    // takes it from glowing-critical to cold and black, so a dead section that
    // is still standing reads as wreckage instead of as something hot. Ramped
    // rather than switched at exactly zero, so it cannot pop.
    let burnt = ((BURNT_BELOW - ratio) / BURNT_BELOW).clamp(0.0, 1.0);
    let base = base.mix(&DEAD_COLOR, burnt);

    let glow = glow_t * (1.0 - burnt);
    let emissive = LinearRgba::new(
        base_emissive.red + GLOW_PEAK.red * glow,
        base_emissive.green + GLOW_PEAK.green * glow,
        base_emissive.blue + GLOW_PEAK.blue * glow,
        base_emissive.alpha,
    );

    (base, emissive)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app with the tint systems wired, for the ECS-level tests.
    ///
    /// Carries the level plugin too: grading reads `DamageLevel` rather than
    /// health directly, so without it every section would read pristine and
    /// every one of these tests would pass for the wrong reason.
    fn tint_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(DamageLevelPlugin);
        // The real fitting path, not a hand-inserted marker: scorch is an
        // AUTHORED effect now, and these tests are only honest if a section
        // gets it the way a spawned one does.
        app.add_plugins(DamageEffectsPlugin { render: true });
        app.init_asset::<StandardMaterial>();
        app.add_systems(
            Update,
            (
                mark_section_meshes,
                resolve_pending_tints,
                grade_section_tints,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn healthy_section_keeps_its_pristine_look() {
        let base = Color::srgb(0.8, 0.8, 0.8);
        let emissive = LinearRgba::BLACK;
        let (color, glow) = damage_look(1.0, base, emissive);
        assert_eq!(color, base);
        assert_eq!(glow, emissive);
    }

    #[test]
    fn damage_reddens_darkens_and_glows_as_integrity_falls() {
        let base = Color::srgb(0.8, 0.8, 0.8);
        let (mid_color, mid_glow) = damage_look(0.5, base, LinearRgba::BLACK);
        let (low_color, low_glow) = damage_look(0.1, base, LinearRgba::BLACK);

        let base_lin = base.to_linear();
        let mid_lin = mid_color.to_linear();
        let low_lin = low_color.to_linear();

        // Reddens: red channel dominates green/blue more as it dies.
        assert!(low_lin.red > low_lin.green);
        assert!(low_lin.green < base_lin.green);
        // Darkens: overall luminance drops with damage.
        assert!(mid_lin.green < base_lin.green);
        assert!(low_lin.green < mid_lin.green);
        // Glow only kicks in below GLOW_BELOW.
        assert_eq!(mid_glow.red, 0.0);
        assert!(low_glow.red > 0.0);
    }

    /// End-to-end through the ECS on the cuboid path: a player-ship section's
    /// rendered material is captured (cloned) and then graded by its `Health`.
    #[test]
    fn grades_a_player_section_material_end_to_end() {
        let mut app = tint_app();

        let pristine = Color::srgb(0.8, 0.8, 0.8);
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: pristine,
                ..default()
            });

        let root = app.world_mut().spawn(Allegiance::Player).id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(root),
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(section)))
            .id();

        // First frames: capture clones the shared material into a private one,
        // grade leaves a full-health section pristine.
        app.update();
        app.update();

        let tint = app
            .world()
            .get::<SectionDamageTint>(mesh)
            .expect("capture should have tagged the section mesh")
            .clone();
        // The mesh no longer points at the shared handle (per-section clone).
        assert_ne!(tint.material.id(), shared.id());
        let graded = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&tint.material)
            .unwrap();
        assert_eq!(
            graded.base_color, pristine,
            "healthy section stays pristine"
        );

        // Damage the section directly and re-grade: the private material reddens
        // and darkens, while the shared source material is untouched.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 10.0;
        app.update();

        let graded = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&tint.material)
            .unwrap()
            .base_color
            .to_linear();
        let pristine_lin = pristine.to_linear();
        assert!(graded.green < pristine_lin.green, "damaged section darkens");
        assert!(graded.red > graded.green, "damaged section reddens");

        let source = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&shared)
            .unwrap();
        assert_eq!(
            source.base_color, pristine,
            "the shared gltf-style source material is never mutated"
        );
    }

    /// An ENEMY section grades exactly like the player's, which is the change:
    /// it used to stay pristine at any health above zero and only black out on
    /// death, hiding how far through it the player had already chewed.
    ///
    /// The split could not survive erosion answering to the same level - a hull
    /// visibly worn through while its paint claimed the section was fine would
    /// be the ship contradicting itself.
    #[test]
    fn an_enemy_section_shows_its_damage_like_any_other() {
        let mut app = tint_app();

        let pristine = Color::srgb(0.8, 0.8, 0.8);
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: pristine,
                ..default()
            });

        let root = app.world_mut().spawn(Allegiance::Enemy).id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(root),
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(section)))
            .id();

        app.update();
        app.update();

        let tint = app
            .world()
            .get::<SectionDamageTint>(mesh)
            .expect("enemy section mesh is captured too")
            .clone();
        let base_of = |app: &App| {
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&tint.material)
                .unwrap()
                .base_color
        };

        // Full health: pristine, on any ship.
        assert_eq!(base_of(&app), pristine, "healthy enemy section is pristine");

        // Heavily damaged but still alive: visibly damaged. THIS is the change
        // - it used to stay pristine here.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 10.0;
        app.update();
        app.update();
        assert_ne!(
            base_of(&app),
            pristine,
            "a chewed-up enemy section must look chewed up"
        );

        // Integrity hits zero: burnt-black, as it always did.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 0.0;
        app.update();
        app.update();
        assert_eq!(
            base_of(&app),
            DEAD_COLOR,
            "a destroyed section still burns out"
        );

        // Inactivity is capability state, not damage. A healthy severed
        // fragment reads by its HEALTH rather than turning black - and now
        // reads the same way on any allegiance.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 100.0;
        app.world_mut()
            .entity_mut(section)
            .insert(SectionInactiveMarker);
        app.update();
        app.update();
        assert_eq!(
            base_of(&app),
            pristine,
            "an intact inactive section keeps its health-derived look"
        );
    }

    /// A fixture's meshes are left out of grading entirely, while a real
    /// section mesh in the same world is still captured.
    ///
    /// Both hang under the same section, so the ChildOf walk reaches it from
    /// either one - being a non-section is not what exempts a plate, stopping
    /// the walk at [`SectionFixture`] is. The cost of getting this wrong is
    /// paid per MESH: a clad ship carries hundreds of plates of one to three
    /// meshes each, and every one of them would clone a `StandardMaterial` to
    /// grade off the health of the hull behind it.
    #[test]
    fn a_fixture_mesh_is_never_captured_for_tinting() {
        let mut app = tint_app();

        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let root = app.world_mut().spawn(Allegiance::Player).id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(root),
            ))
            .id();
        let section_mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(section)))
            .id();
        // A plate clad to that section, with its own surface mesh under it.
        let fixture = app
            .world_mut()
            .spawn((
                SectionFixture,
                Health {
                    current: 40.0,
                    max: 40.0,
                },
                ChildOf(section),
            ))
            .id();
        let fixture_mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(fixture)))
            .id();

        app.update();
        app.update();

        assert!(
            app.world().get::<SectionDamageTint>(section_mesh).is_some(),
            "a real section mesh is still graded",
        );
        assert!(
            app.world().get::<SectionDamageTint>(fixture_mesh).is_none(),
            "a plate must not redden - it comes OFF instead",
        );
        assert!(
            app.world()
                .get::<PendingSectionTint>(fixture_mesh)
                .is_none(),
            "a fixture mesh must not sit pending either, retried every frame",
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(fixture_mesh)
                .expect("the fixture mesh keeps its material")
                .0
                .id(),
            shared.id(),
            "a fixture mesh keeps the shared material rather than cloning one",
        );
    }

    /// Scorch is AUTHORED, and a section that does not author it must stay the
    /// colour it was painted.
    ///
    /// The load-bearing half is the material handle: a section left out of
    /// grading must not be given a private clone either, or every unscorched
    /// section in a fleet would still cost one material for nothing.
    #[test]
    fn a_section_that_authors_no_scorch_is_never_graded() {
        let mut app = tint_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let pristine = app
            .world_mut()
            .spawn((
                SectionMarker,
                DamageEffects::none(),
                Health {
                    current: 10.0,
                    max: 100.0,
                },
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(pristine)))
            .id();

        app.update();
        app.update();

        assert!(
            app.world().get::<SectionDamageTint>(mesh).is_none(),
            "a section that authors no scorch is not graded",
        );
        assert!(
            app.world().get::<PendingSectionTint>(mesh).is_none(),
            "and does not sit pending, retried every frame",
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                .expect("the mesh keeps its material")
                .0
                .id(),
            shared.id(),
            "it keeps the shared material rather than paying for a clone",
        );
    }

    /// A section that does not scorch must not fall through to the section
    /// ABOVE it: a turret authoring no scorch would otherwise have its meshes
    /// graded by the hull it is bolted to, reporting a pool that is not its
    /// own. Wrong information is worse than none.
    #[test]
    fn an_unscorched_section_does_not_borrow_its_parents_health() {
        let mut app = tint_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let hull = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 10.0,
                    max: 100.0,
                },
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                SectionMarker,
                DamageEffects::none(),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(hull),
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(turret)))
            .id();

        app.update();
        app.update();

        assert!(
            app.world().get::<SectionDamageTint>(mesh).is_none(),
            "the walk stops at the unscorched section, it does not carry on up",
        );
    }

    /// A DECORATION's meshes are exempt too, and they hang one level further
    /// out than a plate's: mesh, greeble, plate, section.
    ///
    /// The plate was the first fixture kind and this is the second, so the walk
    /// is checked at the depth the new one actually stands at. A greeble's model
    /// is a loaded glTF scene, so its meshes are spawned by the scene loader
    /// under the fixture rather than authored beside it, and every one of them
    /// arrives through this system.
    #[test]
    fn a_decoration_mesh_under_a_plate_is_never_captured_either() {
        let mut app = tint_app();

        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let root = app.world_mut().spawn(Allegiance::Player).id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(root),
            ))
            .id();
        let plate = app
            .world_mut()
            .spawn((SectionFixture, ChildOf(section)))
            .id();
        let greeble = app.world_mut().spawn((SectionFixture, ChildOf(plate))).id();
        // What the scene loader hangs under a greeble: a mesh two fixtures deep.
        let greeble_mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(greeble)))
            .id();

        app.update();
        app.update();

        assert!(
            app.world().get::<SectionDamageTint>(greeble_mesh).is_none(),
            "a greeble must not redden with the hull it is bolted to",
        );
        assert!(
            app.world()
                .get::<PendingSectionTint>(greeble_mesh)
                .is_none(),
            "a greeble mesh must not sit pending, retried every frame",
        );
    }

    /// R1.1: a section mesh whose material asset is not yet loaded (async gltf)
    /// must stay pending and be captured once the asset arrives, not dropped.
    #[test]
    fn capture_rearms_until_material_asset_loads() {
        let mut app = tint_app();

        // A handle to a material that is NOT in `Assets` yet - the async-gltf
        // situation where the mesh has a material handle before its asset loads.
        let handle = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .reserve_handle();

        let root = app.world_mut().spawn(Allegiance::Player).id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(root),
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(handle.clone()), ChildOf(section)))
            .id();

        // Asset still missing: the mesh is marked pending but not yet captured.
        app.update();
        app.update();
        assert!(
            app.world().get::<SectionDamageTint>(mesh).is_none(),
            "must not capture before the material asset exists"
        );
        assert!(
            app.world().get::<PendingSectionTint>(mesh).is_some(),
            "must stay pending, re-arming, until the asset loads"
        );

        // The asset arrives; the next frame captures it.
        let pristine = Color::srgb(0.3, 0.6, 0.9);
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(
                &handle,
                StandardMaterial {
                    base_color: pristine,
                    ..default()
                },
            )
            .expect("insert material asset");
        app.update();

        let tint = app
            .world()
            .get::<SectionDamageTint>(mesh)
            .expect("must capture once the asset loads");
        assert_eq!(tint.base_color, pristine);
        assert!(
            app.world().get::<PendingSectionTint>(mesh).is_none(),
            "pending marker is cleared after capture"
        );
    }

    /// Regression: a ship exploding chain-destroys its section leaves, and
    /// `mark_section_meshes`
    /// had queued a `PendingSectionTint` insert on a section mesh that gains its
    /// material the SAME frame. The deferred insert then landed on a despawned
    /// entity and panicked. This mirrors the frame order - a despawn queued
    /// ahead of `mark`'s buffer (via `chain_ignore_deferred`, so no sync point
    /// separates them) - with the game's panic-on-command-error fallback set, so
    /// it panics without the `try_insert` guard and passes with it.
    #[test]
    fn tinting_a_section_mesh_chain_destroyed_the_same_frame_does_not_panic() {
        use bevy::ecs::error::{panic, FallbackErrorHandler};

        #[derive(Resource)]
        struct Doomed(Entity);
        fn despawn_doomed(mut commands: Commands, doomed: Res<Doomed>) {
            commands.entity(doomed.0).despawn();
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<StandardMaterial>();
        // Match the game/binary: a command error on a despawned entity is a hard
        // panic, not a silent warn.
        app.insert_resource(FallbackErrorHandler(panic));
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let root = app.world_mut().spawn(Allegiance::Enemy).id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(root),
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(section)))
            .id();
        app.insert_resource(Doomed(mesh));
        // The crash order: `despawn_doomed` queues the despawn, then
        // `mark_section_meshes` (its query still sees the live mesh) queues the
        // tint insert; with no sync point between, the despawn applies first and
        // `mark`'s insert lands on the despawned mesh.
        app.add_systems(
            Update,
            (despawn_doomed, mark_section_meshes).chain_ignore_deferred(),
        );

        app.update();

        assert!(
            app.world().get_entity(mesh).is_err(),
            "the section mesh was chain-destroyed this frame"
        );
    }
}
