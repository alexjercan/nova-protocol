//! Diegetic hull integrity: a ship IS its own health readout.
//!
//! Instead of a generic screen-space health bar, each ship section's rendered
//! material is graded by that section's `Health`. Grading has two modes, chosen
//! by the ship root's [`Allegiance`]:
//!
//! - [`TintMode::Full`] (the player ship, `Allegiance::Player`): a healthy
//!   section keeps its authored look, a battered one reddens and darkens, and a
//!   dead or disabled one reads burnt. This surfaces the per-section integrity
//!   the aggregate bar flattened away and fills the damaged-but-alive gap that
//!   only death (the explode pipeline) otherwise reveals.
//! - [`TintMode::DeadOnly`] (enemy ships, `Allegiance::Enemy`): only the burnt
//!   endpoint. A section stays pristine while it has any integrity and blacks
//!   out the moment it is destroyed or disabled - no intermediate red or glow.
//!   This gives the player a quick "which of their components have I knocked
//!   out" read without turning the enemy into a full health gauge.
//!
//! Neutral / unmarked bodies are never tinted.
//!
//! - Task (player, v1): tasks/20260717-003613/TASK.md
//! - Task (enemy dead-only): tasks/20260718-181305/TASK.md
//! - Spike: tasks/20260711-202901/SPIKE.md (Option 1, recommended)
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
use bevy_common_systems::prelude::Health;

use crate::prelude::{Allegiance, SectionInactiveMarker, SectionMarker};

/// Glob-import surface: `use nova_gameplay::sections::damage_tint::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{SectionDamageTint, SectionDamageTintPlugin, TintMode};
}

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
/// Peak red emissive glow added at zero integrity (before death).
const GLOW_PEAK: LinearRgba = LinearRgba::new(2.2, 0.18, 0.05, 1.0);

/// How a captured section mesh is graded from its section's integrity, selected
/// by the owning ship's [`Allegiance`] at capture time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TintMode {
    /// Full diegetic gradient: redden, darken and glow as integrity falls, then
    /// burnt-black when destroyed. Used for the player ship.
    Full,
    /// Only the burnt-black endpoint: pristine while the section has any
    /// integrity, `DEAD_COLOR` once it is destroyed or disabled. No intermediate
    /// red or glow. Used for enemy ships.
    DeadOnly,
}

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
    /// How this mesh grades from integrity (player full gradient vs enemy
    /// dead-only), fixed by the owning ship's allegiance at capture time.
    pub mode: TintMode,
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
    mode: TintMode,
}

/// Walk up the `ChildOf` chain from `entity` to the nearest ancestor that is a
/// section, returning that section entity. Returns `None` if the walk leaves the
/// tree without passing through a `SectionMarker`.
fn owning_section(
    entity: Entity,
    q_child_of: &Query<&ChildOf>,
    q_is_section: &Query<(), With<SectionMarker>>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if q_is_section.get(current).is_ok() {
            return Some(current);
        }
        current = q_child_of.get(current).ok()?.0;
    }
}

/// Tag every freshly-spawned ship section mesh for tint capture, recording the
/// grading mode its owning ship's allegiance selects. The ChildOf walk and the
/// allegiance gate happen here, once per mesh; the actual material clone is
/// deferred to `resolve_pending_tints` so a not-yet-loaded asset does not drop
/// the mesh.
#[allow(clippy::type_complexity)]
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
    q_allegiance: Query<&Allegiance>,
) {
    for entity in &q_new {
        let Some(section) = owning_section(entity, &q_child_of, &q_is_section) else {
            continue;
        };

        // The section is a direct child of its ship root; the root's `Allegiance`
        // (required by both the player and AI ship markers) picks the grading
        // mode. Player ships grade fully, enemy ships only black out on death,
        // and neutral / unmarked bodies are not diegetic at all.
        let Ok(root) = q_child_of.get(section).map(|c| c.0) else {
            continue;
        };
        let mode = match q_allegiance.get(root) {
            Ok(Allegiance::Player) => TintMode::Full,
            Ok(Allegiance::Enemy) => TintMode::DeadOnly,
            Ok(Allegiance::Neutral) | Err(_) => continue,
        };

        // `try_insert`, not `insert`: a section mesh can be chain-destroyed the
        // same frame it gains its material (a ship exploding), despawning this
        // entity before the buffer applies - the insert must be a no-op there,
        // not a panic (Rust Tally crash, task 20260721-224506).
        commands
            .entity(entity)
            .try_insert(PendingSectionTint { section, mode });
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

        // Same despawn race as `mark_section_meshes`: a pending section mesh can
        // be chain-destroyed before this resolves, so tolerate a missing entity
        // rather than panic (task 20260721-224506).
        commands
            .entity(entity)
            .try_insert((
                MeshMaterial3d(handle.clone()),
                SectionDamageTint {
                    section: pending.section,
                    material: handle,
                    base_color,
                    emissive,
                    mode: pending.mode,
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
    q_health: Query<(&Health, Has<SectionInactiveMarker>), With<SectionMarker>>,
) {
    for tint in &q_tints {
        let (base_color, emissive) = match q_health.get(tint.section) {
            // Destroyed or disabled: burnt-black in both modes.
            Ok((_, true)) => (DEAD_COLOR, tint.emissive),
            Ok((health, false)) => {
                let ratio = if health.max > 0.0 {
                    (health.current / health.max).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                match tint.mode {
                    // Player: the full redden/darken/glow gradient.
                    TintMode::Full => damage_look(ratio, tint.base_color, tint.emissive),
                    // Enemy: pristine until integrity hits zero, then burnt.
                    // No intermediate red or glow - only the black endpoint.
                    TintMode::DeadOnly if ratio <= 0.0 => (DEAD_COLOR, tint.emissive),
                    TintMode::DeadOnly => (tint.base_color, tint.emissive),
                }
            }
            // Section gone (mid-despawn) or lost its Health: leave pristine.
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
    let emissive = LinearRgba::new(
        base_emissive.red + GLOW_PEAK.red * glow_t,
        base_emissive.green + GLOW_PEAK.green * glow_t,
        base_emissive.blue + GLOW_PEAK.blue * glow_t,
        base_emissive.alpha,
    );

    (base, emissive)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app with the tint systems wired, for the ECS-level tests.
    fn tint_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
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

    /// An enemy-ship section (`Allegiance::Enemy` -> `TintMode::DeadOnly`) stays
    /// pristine at full AND partial health - no intermediate red - and blacks out
    /// only when its integrity reaches zero or it is disabled.
    #[test]
    fn enemy_section_blacks_out_only_when_destroyed_never_reddens() {
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
        assert_eq!(tint.mode, TintMode::DeadOnly, "enemy grades dead-only");

        let base_of = |app: &App| {
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&tint.material)
                .unwrap()
                .base_color
        };

        // Full health: pristine.
        assert_eq!(base_of(&app), pristine, "healthy enemy section is pristine");

        // Heavily damaged but still alive: STILL pristine (no red, unlike the
        // player's full gradient at the same ratio).
        app.world_mut().get_mut::<Health>(section).unwrap().current = 10.0;
        app.update();
        assert_eq!(
            base_of(&app),
            pristine,
            "damaged-but-alive enemy section shows no intermediate red"
        );

        // Integrity hits zero: burnt-black.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 0.0;
        app.update();
        assert_eq!(
            base_of(&app),
            DEAD_COLOR,
            "destroyed enemy section blacks out"
        );

        // A disabled (but not yet zero-HP) section also reads burnt.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 50.0;
        app.world_mut()
            .entity_mut(section)
            .insert(SectionInactiveMarker);
        app.update();
        assert_eq!(
            base_of(&app),
            DEAD_COLOR,
            "disabled enemy section blacks out even with residual HP"
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

    /// Regression for the Rust Tally crash (task 20260721-224506): a ship
    /// exploding chain-destroys its section leaves, and `mark_section_meshes`
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
