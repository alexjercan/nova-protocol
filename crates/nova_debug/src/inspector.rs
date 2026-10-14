//! The `bevy_egui` plumbing the Nova Debug window draws on, plus the avian
//! physics gizmos and their diagnostics UI.
//!
//! The gizmos and the diagnostics panel are the `physics` flag of
//! [`DebugOverlays`](crate::hud::overlays::DebugOverlays); the window is
//! [`DebugHudPlugin`](crate::hud::DebugHudPlugin).

use avian3d::prelude::*;
use bevy::{camera::RenderTarget, prelude::*, winit::WinitPlugin};
use bevy_inspector_egui::{
    bevy_egui,
    bevy_egui::{EguiContext, EguiMultipassSchedule, EguiPlugin, PrimaryEguiContext},
    DefaultInspectorConfigPlugin,
};

/// `bevy_egui`, the inspector's reflection config, and avian's physics
/// gizmos and diagnostics.
///
/// The egui half is added only when [`WinitPlugin`] is present; see
/// [`add_egui`]. The avian gizmos and diagnostics are added unconditionally.
/// Add it before [`DebugHudPlugin`](crate::hud::DebugHudPlugin), which draws
/// only when egui is there.
pub struct InspectorDebugPlugin;

impl Plugin for InspectorDebugPlugin {
    fn build(&self, app: &mut App) {
        add_egui(app);

        app.add_plugins((
            avian3d::prelude::PhysicsDebugPlugin,
            PhysicsDiagnosticsPlugin,
            PhysicsDiagnosticsUiPlugin,
        ));
    }
}

/// Add `bevy_egui` and the primary-context placement to an app that has a
/// winit event loop.
///
/// Winit is the precondition, not rendering: egui takes its input from the
/// winit window behind each `Window` entity. `AppBuilder::headless` disables
/// `WinitPlugin` while a virtual `(Window, PrimaryWindow)` entity still
/// exists, so an assembled `bevy_egui` warns once per window per frame
/// ("Cannot access an underlying winit window") and drives a window nobody can
/// see. Keying on winit also covers an offscreen run that has a GPU but no
/// window.
fn add_egui(app: &mut App) {
    if !app.is_plugin_added::<WinitPlugin>() {
        return;
    }

    app.add_plugins(EguiPlugin::default());
    app.add_plugins(DefaultInspectorConfigPlugin);

    // Auto-creation stays off - `keep_inspector_on_window_camera`
    // owns primary-context placement, and both would fight over it.
    app.insert_resource(bevy_egui::EguiGlobalSettings {
        auto_create_primary_context: false,
        ..Default::default()
    });
    app.add_systems(Update, keep_inspector_on_window_camera);
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

    /// The headless regression: `AppBuilder::headless` disables `WinitPlugin`
    /// but still carries a virtual `Window` entity, so an assembled
    /// `bevy_egui` warns about the winit window it cannot reach on every
    /// frame. Only the winit-less direction is pinned here - `WinitPlugin`
    /// builds a real event loop, which a test has no display for.
    #[test]
    fn an_app_without_winit_never_assembles_egui() {
        let mut app = App::new();
        add_egui(&mut app);

        assert!(
            !app.is_plugin_added::<EguiPlugin>(),
            "a winit-less app must not assemble bevy_egui"
        );
        assert!(
            app.world()
                .get_resource::<bevy_egui::EguiGlobalSettings>()
                .is_none(),
            "nothing that configures egui may land either"
        );
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

/// `InspectorDebugPlugin`.
pub mod prelude {
    pub use super::InspectorDebugPlugin;
}
