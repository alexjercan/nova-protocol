use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui,
    bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext},
    egui, DefaultInspectorConfigPlugin,
};

/// The key that toggles debug mode on and off.
pub const DEBUG_TOGGLE_KEYCODE: KeyCode = KeyCode::F11;

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
/// - A hotkey (F11) to toggle all debug features
///
/// The inspector window behaves similarly to the WorldInspectorPlugin
/// but is driven by a custom UI system.
pub struct InspectorDebugPlugin;

impl Plugin for InspectorDebugPlugin {
    fn build(&self, app: &mut App) {
        // Start with debug mode enabled.
        app.insert_resource(DebugEnabled(true));

        // Add the Egui plugin and enable Bevy Inspector defaults.
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(DefaultInspectorConfigPlugin);

        // Render inspector UI only when debug mode is enabled.
        app.add_systems(
            EguiPrimaryContextPass,
            inspector_ui.run_if(resource_equals(DebugEnabled(true))),
        );

        // Disable auto creation of the primary Egui context.
        // We want to assign it manually when cameras are added.
        app.insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..Default::default()
        });

        // Add observer so that new cameras automatically get the PrimaryEguiContext.
        app.add_observer(on_add_camera);

        // Physics debug plugins.
        app.add_plugins((
            avian3d::prelude::PhysicsDebugPlugin::default(),
            PhysicsDiagnosticsPlugin,
            PhysicsDiagnosticsUiPlugin,
        ));

        // Update debug state each frame.
        app.add_systems(
            Update,
            (enable_physics_gizmos, enable_physics_ui, toggle_debug_mode),
        );
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
            // Full world inspector.
            bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);

            // Materials section.
            egui::CollapsingHeader::new("Materials").show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_assets::<StandardMaterial>(world, ui);
            });

            // Entity explorer.
            ui.heading("Entities");
            bevy_inspector_egui::bevy_inspector::ui_for_entities(world, ui);
        });
    });
}

/// When a camera is added, assign it the PrimaryEguiContext so it can display UI.
fn on_add_camera(
    add: On<Add, Camera>,
    mut commands: Commands,
    q_context: Query<&PrimaryEguiContext>,
) {
    let entity = add.entity;
    debug!("on_add_camera: entity {:?}", entity);

    if !q_context.is_empty() {
        debug!("on_add_camera: PrimaryEguiContext already exists, skipping");
        return;
    }

    commands.entity(entity).insert(PrimaryEguiContext);
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

/// Toggle DebugEnabled when the debug toggle key is pressed.
fn toggle_debug_mode(mut debug: ResMut<DebugEnabled>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(DEBUG_TOGGLE_KEYCODE) {
        **debug = !**debug;
    }
}
