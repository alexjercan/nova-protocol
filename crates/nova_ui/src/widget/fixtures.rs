//! Test fixtures shared by the widget-family test modules.

use std::sync::{Arc, Mutex};

use bevy::{
    camera::NormalizedRenderTarget,
    log::tracing_subscriber::{self, util::SubscriberInitExt},
    picking::{
        backend::HitData,
        events::{Pointer, Press},
        pointer::{Location, PointerButton, PointerId},
    },
    prelude::*,
    window::WindowRef,
};

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

/// Spawn a window for presses to bubble up to.
pub(super) fn spawn_window(app: &mut App) -> Entity {
    app.world_mut().spawn(Window::default()).id()
}

/// Press `entity` with the left button, as picking does: the press bubbles up
/// the tree and ends on `window`.
pub(super) fn press(app: &mut App, entity: Entity, window: Entity) {
    let target = NormalizedRenderTarget::Window(
        WindowRef::Entity(window)
            .normalize(None)
            .expect("a named window normalizes"),
    );
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        Location {
            target,
            position: Vec2::ZERO,
        },
        Press {
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        },
        entity,
    ));
    app.world_mut().flush();
}

/// A shared buffer the test log writes to.
#[derive(Clone, Default)]
pub(super) struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl CapturedLog {
    /// Everything logged so far.
    pub(super) fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("the log is not poisoned")).into_owned()
    }

    /// Route this thread's log into the buffer until the guard drops.
    ///
    /// `despawn` and `remove` bake the WARN handler in when they are queued, so
    /// a double one shows up here and nowhere else.
    pub(super) fn capture(&self) -> impl Drop {
        let writer = self.clone();
        tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default()
    }
}

impl std::io::Write for CapturedLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("the log is not poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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
