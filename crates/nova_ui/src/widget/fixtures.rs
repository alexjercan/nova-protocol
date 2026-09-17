//! Test fixtures shared by the widget-family test modules.

use bevy::prelude::*;

use crate::{
    theme::{
        base::base_ui_themes,
        config::{HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        resolve::resolve_theme,
        ActiveUiTheme, GameUiThemes, SelectedUiTheme,
    },
    NovaUiPlugin,
};

/// A headless app with the widget observers + theme reconcilers registered, the
/// two base themes registered as content, and one of them selected - so
/// `app.update()` drives the live paint and `select(&mut app, ..)` drives a
/// theme flip the way Settings does.
pub(super) fn themed_app(theme_id: &str) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, NovaUiPlugin));
    app.insert_resource(GameUiThemes(base_ui_themes()));
    app.insert_resource(SelectedUiTheme(theme_id.to_string()));
    app
}

/// Switch the app's theme the way the Settings row does: move the selection and
/// let the resolver publish the new [`ActiveUiTheme`].
pub(super) fn select(app: &mut App, theme_id: &str) {
    *app.world_mut().resource_mut::<SelectedUiTheme>() = SelectedUiTheme(theme_id.to_string());
}

/// The phosphor theme resolved on its own, for asserting an expected colour
/// without restating a hex the builders already carry.
pub(super) fn phosphor() -> ActiveUiTheme {
    resolve_theme(PHOSPHOR_THEME_ID, &base_ui_themes()).expect("base phosphor resolves")
}

/// The hardware theme resolved on its own.
pub(super) fn hardware() -> ActiveUiTheme {
    resolve_theme(HARDWARE_THEME_ID, &base_ui_themes()).expect("base hardware resolves")
}

/// An entity's background colour.
pub(super) fn bg(app: &App, entity: Entity) -> Color {
    app.world()
        .entity(entity)
        .get::<BackgroundColor>()
        .unwrap()
        .0
}

/// Whether an entity carries a bevel gradient.
pub(super) fn has_gradient(app: &App, entity: Entity) -> bool {
    app.world().entity(entity).contains::<BackgroundGradient>()
}
