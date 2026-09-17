//! Shared themed widgets, painted from the live [`ActiveUiTheme`] - whichever
//! theme the enabled mods declared and the player selected.
//!
//! The heart is the [`ThemedButton`] (`button`): one click + colour model for
//! every screen (menu, editor, HUD chrome). Small chrome helpers (`chrome`)
//! render the rest.
//!
//! NO factory here takes paint, and none reads the theme: every widget spawns
//! UNPAINTED carrying a marker, and its reconciler is the single paint path.
//! That is what lets a theme change reach what is already on screen, and it is
//! also why a caller never threads a look through a hundred spawn sites.
//!
//! Every reconciler runs `.after(UiThemeSystems)`, so a selection change
//! resolves and repaints in ONE frame rather than flashing the old theme.
//!
//! The slider track is the odd family out: a theme chooses between a segmented
//! block-meter and a solid fill, which are different widgets rather than
//! different colours, so it REBUILDS its children where the others repaint.
//!
//! One family per module - `button`, `panel`, `list_row`, `slider`,
//! `segmented`, `chrome` - all re-exported here, so `widget::<item>` paths are
//! unchanged. [`NovaUiPlugin`](crate::NovaUiPlugin) wires every family's
//! observers and reconcilers.

/// Glob-import surface for the themed widgets: the button family and its
/// selection machinery, the panel/list-row/segmented/slider families, and the
/// small layout helpers.
pub mod prelude {
    pub use super::{
        badge, button, button_on_setting, checkbox, checkbox_glyph, key_chip, list_row,
        list_row_colors, menu_button, panel, panel_head, panel_header, panel_node, panel_radius,
        segmented, segmented_container, segmented_container_wrapping, segmented_option,
        segmented_option_fit, separator, slider_meter_color, slider_track, swatch, text_field,
        themed_button, toggle, BadgeKind, ButtonLabel, ButtonSpec, ButtonValue, ButtonVariant,
        KeyChip, KeyChipLabel, ListRow, PanelHeadTag, PanelHeadTitle, SegmentedSkin, Selected,
        SliderBlock, SliderFill, SliderTrack, TextField, TextFieldError, TextFieldFocused,
        TextFieldSpec, TextFieldSubmitted, TextFieldSystems, TextFieldValue, ThemedBadge,
        ThemedBorder, ThemedButton, ThemedCheckbox, ThemedFill, ThemedImageTint, ThemedPanel,
        ThemedPanelHead, ThemedRadius, ThemedSeparator, ThemedSwatch, ThemedText, ThemedTextShadow,
        ThemedToggle, UiText,
    };
}

mod button;
mod chrome;
mod list_row;
mod panel;
mod segmented;
mod slider;
mod text_field;

#[cfg(test)]
mod fixtures;

use bevy::{
    input::keyboard::KeyboardInput,
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
pub use text_field::*;

use self::{
    button::{button_on_interaction, reconcile_button_themes, reconcile_key_chips},
    chrome::{
        reconcile_badges, reconcile_checkboxes, reconcile_separators, reconcile_swatches,
        reconcile_themed_images, reconcile_themed_nodes, reconcile_themed_text,
        reconcile_themed_text_shadows, reconcile_toggles,
    },
    list_row::{list_row_on_interaction, reconcile_list_row_themes},
    panel::{reconcile_panel_head_themes, reconcile_panel_themes},
    segmented::reconcile_segmented_themes,
    slider::{reconcile_slider_track_themes, sync_slider_tracks},
    text_field::{
        one_field_holds_the_focus, paint_text_fields, text_field_keyboard, text_field_on_pointer,
    },
};
use crate::{
    font::UiFont,
    input_mode::{owns_or_enters, InputMode},
    theme::UiThemeSystems,
};

/// Marks the currently-active button within a `ButtonValue<T>` selection group.
#[derive(Component)]
pub struct Selected;

/// Marks any text span whose face should route through [`UiFont`] (Iosevka
/// Term). `apply_ui_font` fills the handle once the resource exists.
#[derive(Component)]
pub struct UiText;

/// Wire the button colour observers, the theme reconcilers and the font router.
pub(crate) fn build(app: &mut App) {
    app.add_message::<KeyboardInput>();
    app.add_message::<TextFieldSubmitted>();

    app.add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Add, InteractionDisabled>)
        .add_observer(button_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(button_on_interaction::<Insert, Hovered>)
        .add_observer(button_on_interaction::<Add, Selected>)
        .add_observer(button_on_interaction::<Remove, Selected>);

    // Bevy's own commit of a dragged slider onto its `SliderValue`. It belongs
    // beside the track that SHOWS that value: every caller of `slider_track`
    // needs it, and two callers registering it would run it twice per drag.
    app.add_observer(bevy::ui_widgets::slider_self_update);

    app.add_observer(list_row_on_interaction::<Insert, Hovered>)
        .add_observer(list_row_on_interaction::<Add, Selected>)
        .add_observer(list_row_on_interaction::<Remove, Selected>)
        .add_observer(text_field_on_pointer);

    app.add_systems(
        Update,
        (
            reconcile_button_themes,
            reconcile_key_chips,
            reconcile_list_row_themes,
            reconcile_panel_themes,
            reconcile_panel_head_themes,
            reconcile_segmented_themes,
            reconcile_themed_text,
            reconcile_themed_text_shadows,
            reconcile_themed_images,
            reconcile_themed_nodes,
            reconcile_separators,
            reconcile_badges,
            reconcile_checkboxes,
            reconcile_swatches,
            reconcile_toggles,
            // The rebuild first, then the value onto its new children. The
            // explicit edge auto-inserts an `ApplyDeferred`
            // (`ScheduleBuildSettings::auto_insert_apply_deferred`, default
            // true), so the respawned children are VISIBLE to the value system
            // this same frame.
            reconcile_slider_track_themes,
            sync_slider_tracks.after(reconcile_slider_track_themes),
            (
                one_field_holds_the_focus.before(text_field_keyboard),
                // The field is Insert's owner, so it types under Insert and
                // under Normal - the frame a click gives it the caret, the
                // arbiter has not resolved yet. What it does NOT do is type
                // under a mode above its own: a keybind capture takes the
                // keyboard off a focused field like it takes it off everything
                // else.
                text_field_keyboard.run_if(owns_or_enters(InputMode::Insert)),
                paint_text_fields,
            )
                .in_set(TextFieldSystems),
        )
            // The theme resolves first: a reconciler that ran BEFORE it painted
            // the outgoing theme, so every selection change showed one stale
            // frame.
            .after(UiThemeSystems),
    );
    // Route the font BEFORE UI text is measured/laid out (PostUpdate,
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
