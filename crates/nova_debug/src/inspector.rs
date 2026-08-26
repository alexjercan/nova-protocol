//! The egui world inspector, the avian physics gizmos and their diagnostics UI,
//! toggled together as one debug panel.
//!
//! Nova owns this because the inspector is one of the four layers
//! [`crate::DebugPlugin`] raises together on F11. The F11 key itself is read
//! ONCE, by `DebugPlugin`, which also overrides this `DebugEnabled` with
//! `DEBUG_LAYER_STARTS_ON` so the layer boots OFF; `sync_inspector_cursor`
//! hands the cursor back while the panel is up.

use avian3d::prelude::*;
use bevy::{camera::RenderTarget, prelude::*};
use bevy_inspector_egui::{
    bevy_egui,
    bevy_egui::{
        EguiContext, EguiMultipassSchedule, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext,
    },
    egui, DefaultInspectorConfigPlugin,
};

/// Resource that stores whether debug mode is enabled.
///
/// When true, the inspector UI, physics gizmos, and diagnostics UI are visible.
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct DebugEnabled(pub bool);

/// A plugin that provides a full debug UI and physics visualization.
///
/// This plugin adds:
/// - Egui support
/// - An inspector window for inspecting the world, entities, and assets
/// - Physics debug gizmos from avian3d
/// - Physics diagnostics and their UI
///
/// The inspector window behaves similarly to the WorldInspectorPlugin
/// but is driven by a custom UI system.
pub struct InspectorDebugPlugin;

impl Plugin for InspectorDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugEnabled(true));

        app.add_plugins(EguiPlugin::default());
        app.add_plugins(DefaultInspectorConfigPlugin);

        app.add_systems(
            EguiPrimaryContextPass,
            inspector_ui.run_if(resource_equals(DebugEnabled(true))),
        );

        // Auto-creation stays off - `keep_inspector_on_window_camera`
        // owns primary-context placement, and both would fight over it.
        app.insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..Default::default()
        });
        app.add_systems(Update, keep_inspector_on_window_camera);

        app.add_plugins((
            avian3d::prelude::PhysicsDebugPlugin,
            PhysicsDiagnosticsPlugin,
            PhysicsDiagnosticsUiPlugin,
        ));

        app.add_systems(Update, (enable_physics_gizmos, enable_physics_ui));
    }
}

/// Draws the inspector UI when debug mode is enabled.
///
/// This creates a window with:
/// - Full world inspector
/// - Material inspector
/// - Entity list and explorer
///
/// The UI uses the same internal systems as WorldInspectorPlugin.
fn inspector_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        error!("inspector_ui: no EguiContext found");
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("Debug Inspector").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);

            egui::CollapsingHeader::new("Materials").show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_assets::<StandardMaterial>(world, ui);
            });

            ui.heading("Entities");
            bevy_inspector_egui::bevy_inspector::ui_for_entities(world, ui);
        });
    });
}

/// Keep the inspector's `PrimaryEguiContext` on a window-targeting camera,
/// and off any camera that renders to an `Image`.
///
/// A per-frame reconcile instead of an `Add` observer, so the placement is
/// order-independent AND survives retargeting: an `Image`-target camera
/// never owns the context (the inspector would render into the offscreen
/// texture), and when no window camera holds it - first spawn, or the
/// holder just became a render-to-texture camera - the first window camera
/// takes it. Removal and insertion flush together, so there is no frame
/// with zero primary contexts.
///
/// Scope: this plugin owns primary-context placement (auto-creation is
/// disabled); a consumer-placed `PrimaryEguiContext` on a non-camera entity
/// is outside the contract and would duplicate the multipass schedule.
/// "Window" here means any non-`Image` target (`TextureView` included), and
/// "first" is query order, not spawn order - sufficient for the
/// single-window debug tooling this serves.
///
/// Replaces an earlier "first camera added wins" observer, which handed the
/// inspector to whichever camera spawned first - including render-to-texture
/// cameras, where the UI lands inside an offscreen image (a target-inset
/// camera in a downstream game).
fn keep_inspector_on_window_camera(
    mut commands: Commands,
    q_cameras: Query<(Entity, Option<&RenderTarget>, Has<PrimaryEguiContext>), With<Camera>>,
) {
    let renders_to_image =
        |target: Option<&RenderTarget>| matches!(target, Some(RenderTarget::Image(_)));

    let mut window_has_context = false;
    let mut first_window_camera = None;
    for (entity, target, has_context) in &q_cameras {
        if renders_to_image(target) {
            if has_context {
                // Shed the WHOLE egui cluster, not just the marker.
                // `PrimaryEguiContext` requires `EguiContext` (no cascade on
                // removal) and its on_insert hook adds
                // `EguiMultipassSchedule`; leaving either behind means two
                // entities run the same egui schedule and bevy_egui panics.
                commands
                    .entity(entity)
                    .remove::<(PrimaryEguiContext, EguiContext, EguiMultipassSchedule)>();
            }
        } else {
            first_window_camera.get_or_insert(entity);
            window_has_context |= has_context;
        }
    }

    if !window_has_context {
        if let Some(entity) = first_window_camera {
            commands.entity(entity).insert(PrimaryEguiContext);
        }
    }
}

/// Enable or disable physics gizmos based on the DebugEnabled resource.
fn enable_physics_gizmos(mut store: ResMut<GizmoConfigStore>, debug: Res<DebugEnabled>) {
    if debug.is_changed() {
        store
            .config_mut::<avian3d::prelude::PhysicsGizmos>()
            .0
            .enabled = **debug;
    }
}

/// Enable or disable the physics diagnostics UI.
fn enable_physics_ui(mut settings: ResMut<PhysicsDiagnosticsUiSettings>, debug: Res<DebugEnabled>) {
    if debug.is_changed() {
        settings.enabled = **debug;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// App-driven rig for the reconcile alone: no egui/render stack needed -
    /// the system only moves the `PrimaryEguiContext` marker between camera
    /// entities.
    fn rig() -> App {
        let mut app = App::new();
        app.add_systems(Update, keep_inspector_on_window_camera);
        app
    }

    fn rtt_target() -> RenderTarget {
        RenderTarget::Image(Handle::default().into())
    }

    /// The regression the first-camera-wins observer was replaced for: a
    /// render-to-texture camera whose `Add` fires first must not own the
    /// inspector UI.
    #[test]
    fn an_rtt_camera_spawned_first_never_takes_the_context() {
        let mut app = rig();
        let rtt = app
            .world_mut()
            .spawn((Camera::default(), rtt_target()))
            .id();
        let window = app.world_mut().spawn(Camera::default()).id();
        app.update();
        app.update();

        assert!(
            app.world().get::<PrimaryEguiContext>(window).is_some(),
            "the window camera must hold the context"
        );
        assert!(
            app.world().get::<PrimaryEguiContext>(rtt).is_none(),
            "the RTT camera must never hold the context"
        );
    }

    #[test]
    fn the_context_rehomes_when_its_holder_becomes_rtt() {
        let mut app = rig();
        let first = app.world_mut().spawn(Camera::default()).id();
        let second = app.world_mut().spawn(Camera::default()).id();
        app.update();
        app.update();
        assert!(
            app.world().get::<PrimaryEguiContext>(first).is_some(),
            "the first window camera holds the context initially"
        );

        app.world_mut().entity_mut(first).insert(rtt_target());
        app.update();
        app.update();
        assert!(
            app.world().get::<PrimaryEguiContext>(first).is_none(),
            "a camera that became RTT must lose the context"
        );
        assert!(
            app.world().get::<PrimaryEguiContext>(second).is_some(),
            "the remaining window camera must take the context"
        );
    }

    #[test]
    fn no_window_camera_means_no_context_and_no_panic() {
        let mut app = rig();
        let rtt = app
            .world_mut()
            .spawn((Camera::default(), rtt_target()))
            .id();
        app.update();
        app.update();
        assert!(
            app.world().get::<PrimaryEguiContext>(rtt).is_none(),
            "an RTT-only world gets no primary context"
        );
    }
}

#[cfg(test)]
mod rehome_hazard_tests {
    use bevy_inspector_egui::bevy_egui::EnableMultipassForPrimaryContext;

    use super::*;

    /// The multipass composition hazard: `PrimaryEguiContext`'s on_insert hook adds
    /// `EguiMultipassSchedule(EguiPrimaryContextPass)` whenever
    /// `EnableMultipassForPrimaryContext` exists (it does under
    /// `EguiPlugin::default()`), and removing the marker alone leaves the
    /// demoted camera with the schedule (and the required `EguiContext`) -
    /// two entities then run the same egui schedule and bevy_egui panics.
    /// The resource arms the REAL component hook without the full plugin.
    #[test]
    fn a_demoted_holder_sheds_the_whole_egui_cluster() {
        let mut app = App::new();
        app.insert_resource(EnableMultipassForPrimaryContext);
        app.add_systems(Update, keep_inspector_on_window_camera);

        let first = app.world_mut().spawn(Camera::default()).id();
        let second = app.world_mut().spawn(Camera::default()).id();
        app.update();
        app.update();
        assert!(
            app.world().get::<EguiMultipassSchedule>(first).is_some(),
            "the hook must arm the holder's multipass schedule (rig guard)"
        );

        app.world_mut()
            .entity_mut(first)
            .insert(RenderTarget::Image(Handle::default().into()));
        app.update();
        app.update();

        assert!(
            app.world().get::<PrimaryEguiContext>(second).is_some(),
            "the window camera took the context"
        );
        assert!(
            app.world().get::<EguiMultipassSchedule>(first).is_none(),
            "the demoted camera must shed EguiMultipassSchedule, or two \
             entities run the same egui schedule and bevy_egui panics"
        );
        assert!(
            app.world().get::<EguiContext>(first).is_none(),
            "the demoted camera must shed the required EguiContext too"
        );
        let schedules = {
            let mut q = app.world_mut().query::<&EguiMultipassSchedule>();
            q.iter(app.world()).count()
        };
        assert_eq!(schedules, 1, "exactly one multipass schedule holder");
    }
}

/// `InspectorDebugPlugin` and its `DebugEnabled` flag.
pub mod prelude {
    pub use super::{DebugEnabled, InspectorDebugPlugin};
}
