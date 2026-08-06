//! Shared themed widgets, rendered from the NOVA OS palette ([`crate::theme`])
//! in one of two skins ([`crate::skin::UiSkin`]): the phosphor CLI terminal look
//! (default) and the light-3D hardware casing.
//!
//! The heart is the [`ThemedButton`] (`button`): one click + colour model for
//! every screen (menu, editor, HUD chrome). Small layout helpers (`chrome`)
//! render the rest.
//!
//! Each skin-aware widget family carries a marker and rides its own reconciler,
//! so a `UiSkin` flip restyles what is already on screen instead of waiting for
//! the screen to be rebuilt: [`ThemedButton`], [`ListRow`], [`PanelSkin`],
//! [`SegmentedSkin`] and [`SliderTrackSkin`]. The slider track is the odd one -
//! its two skins are structurally different widgets (a row of [`SliderBlock`]s
//! vs one [`SliderFill`]), so it REBUILDS its children where the others repaint.
//!
//! One family per module - `button`, `panel`, `list_row`, `slider`,
//! `segmented`, `chrome` - all re-exported here, so `widget::<item>` paths are
//! unchanged. [`register`] wires every family's observers and reconcilers in
//! one call.

mod button;
mod chrome;
mod list_row;
mod paint;
mod panel;
mod segmented;
mod slider;

#[cfg(test)]
mod fixtures;

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    text::FontSource,
    ui::{InteractionDisabled, Pressed},
};
pub use button::*;
pub use chrome::*;
pub use list_row::*;
pub use panel::*;
pub use segmented::*;
pub use slider::*;

use self::{
    button::{button_on_interaction, reconcile_button_skins},
    list_row::{list_row_on_interaction, reconcile_list_row_skins},
    panel::reconcile_panel_skins,
    segmented::reconcile_segmented_skins,
    slider::{reconcile_slider_track_skins, sync_slider_tracks},
};
use crate::{font::UiFont, skin::UiSkin};

/// Marks the currently-active button within a `ButtonValue<T>` selection group.
#[derive(Component)]
pub struct Selected;

/// Marks any text span whose face should route through [`UiFont`] (Iosevka
/// Term). `apply_ui_font` fills the handle once the resource exists.
#[derive(Component)]
pub struct UiText;

/// Guard resource: the themed-widget observers + systems are app-global, so the
/// first [`register`] call wins and later calls are no-ops (menu and editor both
/// register in the shipped app; doubled observers would write every colour
/// twice per interaction).
#[derive(Resource)]
struct WidgetObserversRegistered;

/// Wire the button colour observers, the skin reconcilers and the font router.
/// Call from each app/plugin that uses themed widgets; guarded, so independent
/// plugins can coexist.
pub fn register(app: &mut App) {
    if app.world().contains_resource::<WidgetObserversRegistered>() {
        return;
    }
    app.insert_resource(WidgetObserversRegistered);
    // NOTE: `init_resource` is idempotent, so owning the skin here keeps the
    // widget layer self-contained for tests and slim apps even though settings
    // is what persists it.
    app.init_resource::<UiSkin>();

    app.add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Add, InteractionDisabled>)
        .add_observer(button_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(button_on_interaction::<Insert, Hovered>)
        .add_observer(button_on_interaction::<Add, Selected>)
        .add_observer(button_on_interaction::<Remove, Selected>);

    app.add_observer(list_row_on_interaction::<Insert, Hovered>)
        .add_observer(list_row_on_interaction::<Add, Selected>)
        .add_observer(list_row_on_interaction::<Remove, Selected>);

    app.add_systems(
        Update,
        (
            reconcile_button_skins,
            reconcile_list_row_skins,
            reconcile_panel_skins,
            reconcile_segmented_skins,
            // NOTE: the rebuild first, then the value onto its new children. The
            // explicit edge auto-inserts an `ApplyDeferred`
            // (`ScheduleBuildSettings::auto_insert_apply_deferred`, default
            // true), so the respawned children are VISIBLE to the value system
            // this same frame.
            reconcile_slider_track_skins,
            sync_slider_tracks.after(reconcile_slider_track_skins),
        ),
    );
    // NOTE: route the font BEFORE UI text is measured/laid out (PostUpdate,
    // before `UiSystems::Content`), not in Update - a `UiText` spawned this
    // frame would otherwise render one frame in the larger default face before
    // Iosevka applies (the "text gets bigger for a split second" flash on any
    // respawn).
    app.add_systems(
        PostUpdate,
        apply_ui_font.before(bevy::ui::UiSystems::Content),
    );
}

/// Route every [`UiText`] span through the shared [`UiFont`] (Iosevka Term) once
/// the resource exists: on the resource landing, set every span; afterwards, set
/// each newly spawned span. A headless rig with no `UiFont` keeps the engine
/// default font.
fn apply_ui_font(
    font: Option<Res<UiFont>>,
    added: Query<Entity, Added<UiText>>,
    mut all: Query<&mut TextFont, With<UiText>>,
) {
    let Some(font) = font else {
        return;
    };
    if font.is_changed() {
        let handle = font.handle();
        for mut tf in &mut all {
            tf.font = FontSource::Handle(handle.clone());
        }
    } else {
        for entity in &added {
            if let Ok(mut tf) = all.get_mut(entity) {
                tf.font = FontSource::Handle(font.handle());
            }
        }
    }
}
