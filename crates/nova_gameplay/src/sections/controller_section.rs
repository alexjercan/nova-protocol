//! A section of a spaceship that can control its rotation using a PD controller.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::{SectionInactiveMarker, SectionRenderOf};

pub mod prelude {
    pub use super::{
        controller_section, preview_controller_section, ControllerSectionConfig,
        ControllerSectionMarker, ControllerSectionPlugin, ControllerSectionRenderMarker,
        ControllerSectionRotationInput, ControllerVerbs, FlightVerb,
    };
}

/// Configuration for a controller section.
#[derive(Clone, Debug, Reflect)]
pub struct ControllerSectionConfig {
    /// The frequency of the PD controller in Hz.
    pub frequency: f32,
    /// The damping ratio of the PD controller.
    pub damping_ratio: f32,
    /// The maximum torque that can be applied by the PD controller.
    pub max_torque: f32,
    /// Which flight verbs this controller grants, its initial loadout. The
    /// autopilot maneuvers (STOP/GOTO/ORBIT) are a capability of the controller
    /// section, so a controller can be authored to withhold one - a cheap
    /// shuttle controller that only brakes, or the shakedown's GOTO-off intro
    /// (spike docs/spikes/20260712-143551-controller-provided-verb-flags.md).
    /// Defaults to all verbs enabled. Scenarios can also flip a verb at runtime
    /// via `SetControllerVerb`.
    pub verbs: ControllerVerbs,
    /// The render mesh of the hull section, defaults to a cuboid of size 1x1x1.
    pub render_mesh: Option<Handle<WorldAsset>>,
}

impl Default for ControllerSectionConfig {
    fn default() -> Self {
        Self {
            frequency: 2.0,
            damping_ratio: 2.0,
            max_torque: 1.0,
            verbs: ControllerVerbs::default(),
            render_mesh: None,
        }
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ControllerSectionRenderMesh(Option<Handle<WorldAsset>>);

/// Helper function to create a controller section entity bundle.
pub fn controller_section(config: ControllerSectionConfig) -> impl Bundle {
    debug!("controller_section: config {:?}", config);

    (
        ControllerSectionMarker,
        PDController {
            frequency: config.frequency,
            damping_ratio: config.damping_ratio,
            max_torque: config.max_torque,
        },
        config.verbs,
        ControllerSectionRotationInput::default(),
        ControllerSectionRenderMesh(config.render_mesh),
    )
}

/// A render-only controller section for the editor preview: it shows the controller mesh (and is
/// pickable) but carries no [`PDController`], so it never tries to torque a root. The editor
/// preview ship is a visual config preview with no `RigidBody`; a live controller there just
/// floods the log with "root not found" every frame (task 20260706-212909). Because it has no
/// `PDController`, the bcs PD systems and `insert_controller_section_target` both skip it, so the
/// preview controller is inert.
pub fn preview_controller_section(config: ControllerSectionConfig) -> impl Bundle {
    debug!("preview_controller_section: config {:?}", config);

    (
        ControllerSectionMarker,
        ControllerSectionRenderMesh(config.render_mesh),
    )
}

/// Marker component for controller sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ControllerSectionMarker;

/// One of the autopilot flight verbs the controller section grants. These are
/// the maneuvers the flight computer can fly (STOP/GOTO/ORBIT); CANCEL is not
/// listed because it only ever disengages an already-running maneuver and stays
/// available so a disabled verb can never strand an engaged autopilot. The enum
/// is the addressable handle used by [`ControllerVerbs`] and the
/// `SetControllerVerb` scenario action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum FlightVerb {
    /// STOP: kill all velocity.
    Stop,
    /// GOTO: fly to the locked target and come to rest.
    Goto,
    /// ORBIT: circularize and station-keep in a gravity well.
    Orbit,
}

/// Per-verb enable flags carried on a controller section: the flight verbs are
/// a capability the controller provides, and each maneuver can be individually
/// withheld while the controller is otherwise alive. A verb is available only
/// if the ship has a live controller section AND that section's flag for the
/// verb is set (layered on top of the existing physical `flyable` gate - a live
/// controller plus a live thruster). Defaults to all verbs enabled. Written at
/// build time from [`ControllerSectionConfig::verbs`] and flipped at runtime by
/// the `SetControllerVerb` scenario action.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct ControllerVerbs {
    /// Whether STOP (kill velocity) is granted.
    pub stop: bool,
    /// Whether GOTO (fly to lock) is granted.
    pub goto: bool,
    /// Whether ORBIT (station-keep in a well) is granted.
    pub orbit: bool,
}

impl Default for ControllerVerbs {
    fn default() -> Self {
        Self {
            stop: true,
            goto: true,
            orbit: true,
        }
    }
}

impl ControllerVerbs {
    /// Whether the given verb is currently granted.
    pub fn granted(&self, verb: FlightVerb) -> bool {
        match verb {
            FlightVerb::Stop => self.stop,
            FlightVerb::Goto => self.goto,
            FlightVerb::Orbit => self.orbit,
        }
    }

    /// Enable or disable the given verb.
    pub fn set(&mut self, verb: FlightVerb, enabled: bool) {
        match verb {
            FlightVerb::Stop => self.stop = enabled,
            FlightVerb::Goto => self.goto = enabled,
            FlightVerb::Orbit => self.orbit = enabled,
        }
    }
}

/// The desired rotation of the controller section, in world space. Written by
/// the player's mouse command, the AI brain, or the autopilot
/// (`crate::flight`) - whoever currently holds rotation authority.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct ControllerSectionRotationInput(pub Quat);

/// A plugin that will enable the ControllerSection.
#[derive(Default)]
pub struct ControllerSectionPlugin {
    pub render: bool,
}

impl Plugin for ControllerSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("ControllerSectionPlugin: build");

        // Register the section's reflected components so the debug inspector
        // (and the flight-feel retune) can see and edit them.
        app.register_type::<ControllerSectionMarker>()
            .register_type::<ControllerSectionRotationInput>()
            .register_type::<ControllerVerbs>()
            .register_type::<FlightVerb>();

        app.add_observer(insert_controller_section_target);

        // The command copy into the bcs PDControllerInput runs on the FIXED
        // clock, between the command writers and the PD (task
        // 20260711-140241): its producer (the autopilot, NovaFlightSystems)
        // and consumer (PDControllerSystems::Sync) both tick in FixedUpdate,
        // and the old Update-schedule copy handed the PD a command 1-2
        // ticks stale, varying with the 64 Hz-vs-render beat - up to
        // 0.22 rad of phantom command error and ~20% wasted torque during
        // autopilot slews. The `after`/`before` pair transitively pins
        // NovaFlightSystems ahead of the PD sync, which the two sets'
        // individual `.before(SpaceshipSectionSystems)` constraints never
        // guaranteed. Update-schedule writers (player mouse, AI brain,
        // torpedo guidance) are unaffected: their command changes once per
        // frame and is picked up by the next tick exactly as before.
        app.add_systems(
            FixedUpdate,
            update_controller_section_rotation_input
                .after(crate::flight::NovaFlightSystems)
                .before(PDControllerSystems::Sync),
        );

        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(super::SpaceshipSectionSystems),
        );

        app.configure_sets(
            FixedUpdate,
            PDControllerSystems::Sync.before(super::SpaceshipSectionSystems),
        );

        if self.render {
            app.add_observer(insert_controller_section_render);
        }
    }
}

// `pub(crate)` so the flight tests can register the real rotation pipeline
// and cover autopilot -> PD -> hull swing end to end.
pub(crate) fn update_controller_section_rotation_input(
    mut q_controller: Query<
        (&mut PDControllerInput, &ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (mut input, desired_rotation) in &mut q_controller {
        **input = **desired_rotation;
    }
}

pub(crate) fn sync_controller_section_forces(
    mut q_root: Query<Forces>,
    // A disabled-in-place controller (zero-health, non-leaf, still attached ->
    // `SectionInactiveMarker`) must stop stabilizing the hull: with no live
    // computer the flight layer's semantics are "adrift" (the autopilot
    // disengages and the player command freezes). Its `PDControllerOutput` is
    // still computed by the bcs PD system, but this is the only seam that
    // applies it, so gating here is what actually stops the torque. Mirrors the
    // filter already on `update_controller_section_rotation_input` and the
    // flight systems.
    q_controller: Query<(&PDControllerOutput, &PDControllerTarget), Without<SectionInactiveMarker>>,
) {
    for (output, target) in &q_controller {
        if let Ok(mut forces) = q_root.get_mut(**target) {
            forces.apply_torque(**output);
        }
    }
}

fn insert_controller_section_target(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    // Only real (live) controllers carry a `PDController`; a render-only preview controller
    // (`preview_controller_section`) does not, so it gets no target and stays inert.
    q_controller: Query<&ChildOf, (With<ControllerSectionMarker>, With<PDController>)>,
) {
    let entity = add.entity;
    trace!("insert_controller_section_target: entity {:?}", entity);
    let Ok(ChildOf(root)) = q_controller.get(entity) else {
        // No `PDController` (a preview controller) - nothing to target. Not an error.
        return;
    };

    commands.entity(entity).insert(PDControllerTarget(*root));
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct ControllerSectionRenderMarker;

fn insert_controller_section_render(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_controller: Query<
        (
            &ControllerSectionRenderMesh,
            Has<ControllerSectionRenderMarker>,
        ),
        With<ControllerSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_controller_section_render: entity {:?}", entity);

    let Ok((render_mesh, has_render)) = q_controller.get(entity) else {
        error!(
            "insert_controller_section_render: entity {:?} not found in q_controller",
            entity
        );
        return;
    };

    if has_render {
        trace!(
            "insert_controller_section_render: entity {:?} already has render, skipping",
            entity
        );
        return;
    }

    commands
        .entity(entity)
        .insert(ControllerSectionRenderMarker);
    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Controller Section Body"),
                SectionRenderOf(entity),
                WorldAssetRoot(scene.clone()),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![
                (
                    Name::new("Controller Section Body (A)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                    MeshMaterial3d(materials.add(Color::srgb(0.2, 0.7, 0.9))),
                ),
                (
                    Name::new("Controller Section Window (B)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cylinder::new(0.2, 0.1))),
                    MeshMaterial3d(materials.add(Color::srgb(0.9, 0.9, 1.0))),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                )
            ],));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_controller_with_default_config() {
        // Arrange
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(controller_section(ControllerSectionConfig::default()))
            .id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_controller_with_custom_scene() {
        // Arrange
        let mut app = App::new();
        let custom_scene = Handle::<WorldAsset>::default();
        let config = ControllerSectionConfig {
            render_mesh: Some(custom_scene.clone()),
            ..Default::default()
        };
        let id = app.world_mut().spawn(controller_section(config)).id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<ControllerSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(render_mesh.0.as_ref().unwrap(), &custom_scene);
    }

    #[test]
    fn preview_controller_carries_no_live_pd_controller() {
        // The editor preview controller renders but must not carry a live PDController - that is
        // what spammed "root not found" against the non-physics preview root (task 20260706-212909).
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(preview_controller_section(
                ControllerSectionConfig::default(),
            ))
            .id();
        app.update();

        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
        assert!(
            app.world().get::<PDController>(id).is_none(),
            "a preview controller must not carry a live PDController"
        );
    }

    #[test]
    fn only_a_live_controller_gets_a_pd_target() {
        // `insert_controller_section_target` gives a target only to controllers that carry a
        // PDController. The bcs PD system iterates `(PDController, ..., PDControllerTarget, ...)`,
        // so a preview controller with neither is never processed and never logs "root not found".
        let mut app = App::new();
        app.add_observer(insert_controller_section_target);

        let root = app.world_mut().spawn_empty().id();
        let live = app
            .world_mut()
            .spawn((
                controller_section(ControllerSectionConfig::default()),
                ChildOf(root),
            ))
            .id();
        let preview = app
            .world_mut()
            .spawn((
                preview_controller_section(ControllerSectionConfig::default()),
                ChildOf(root),
            ))
            .id();
        app.update();

        assert!(
            app.world().get::<PDControllerTarget>(live).is_some(),
            "a live controller targets its root"
        );
        assert!(
            app.world().get::<PDControllerTarget>(preview).is_none(),
            "a preview controller must not target a root - that is the PD-spam fix"
        );
    }
}
