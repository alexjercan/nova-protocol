//! Test fixtures shared by the widget-family test modules.

use bevy::prelude::*;

use super::register;
use crate::skin::UiSkin;

/// A headless app with the widget observers + skin reconcilers registered and
/// the `Update` schedule available, so `app.update()` drives the live paint.
pub(super) fn skin_app(skin: UiSkin) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    register(&mut app);
    app.insert_resource(skin);
    app
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
