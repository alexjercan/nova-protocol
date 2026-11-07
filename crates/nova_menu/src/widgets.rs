//! Shared menu widget primitives: the button constructors every menu surface
//! spawns, and the chip of its back-out key. Every button's voice is
//! `nova_gameplay`'s `ButtonCuePlugin`.

use bevy::prelude::*;
use nova_input::prelude::{gamepad_label, InputDeviceType};
use nova_ui::{
    theme::UiColor,
    widget::{
        menu_button, ButtonCueType, ButtonSpec, ButtonVariant, KeyChipLabel, ThemedText, UiText,
    },
};

/// Marks a [`back_button`], whose key chip names the back-out input of the
/// device the player pressed last.
#[derive(Component)]
pub(crate) struct MenuBackChip;

/// Marks the root of a main-menu screen that sits over the menu card: Settings,
/// Mods, Scenarios, Load and Lessons. Back hides it, and so does Escape
/// ([`back_out_of_the_menu_screen`](crate::back_out::back_out_of_the_menu_screen)).
#[derive(Component)]
pub(crate) struct MenuScreen;

/// Hide a menu screen: the body of every Back button that closes one.
pub(crate) fn close_screen(visibility: &mut Visibility) {
    *visibility = Visibility::Hidden;
}

/// The main-menu / pause / outcome button: the shared nova_ui [`menu_button`] (40px /
/// 16px `ThemedButton`, coloured by the nova_ui observers + theme reconciler).
pub(crate) fn button(text: &str) -> impl Bundle {
    menu_button(text)
}

/// A menu button carrying an emphasis variant (primary call-to-action / danger)
/// and an optional trailing key-chip - the shared nova_ui `ThemedButton` at the
/// 40/16 menu sizing.
pub(crate) fn button_variant(text: &str, variant: ButtonVariant, key: Option<&str>) -> impl Bundle {
    let mut spec = ButtonSpec::new(text).menu();
    spec.variant = variant;
    if let Some(key) = key {
        spec = spec.key(key);
    }
    nova_ui::widget::button(spec)
}

/// A danger button at the default 34px / 14px sizing, the size of a
/// `themed_button` beside it in a details pane. [`button_variant`] is the 40px
/// menu-column sizing.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn danger_button(text: &str) -> impl Bundle {
    nova_ui::widget::button(ButtonSpec::new(text).danger())
}

/// The key a back-out answers to. Escape is fixed by design (see
/// `toggle_pause`), so the chip may name it as a literal.
pub(crate) const BACK_KEY: &str = "Esc";

/// The pad button a back-out answers to, fixed like [`BACK_KEY`].
pub(crate) const BACK_PAD_BUTTON: GamepadButton = GamepadButton::East;

/// Name the back-out input of the device the player pressed last on every
/// [`back_button`] chip: [`BACK_KEY`] at the desk, [`BACK_PAD_BUTTON`] on a
/// pad. Repaints on a device change and on a new button.
pub(crate) fn refresh_back_keys(
    device: Res<InputDeviceType>,
    q_new: Query<(), Added<MenuBackChip>>,
    q_back: Query<Entity, With<MenuBackChip>>,
    q_children: Query<&Children>,
    mut q_label: Query<&mut Text, With<KeyChipLabel>>,
) {
    if !device.is_changed() && q_new.is_empty() {
        return;
    }
    let key = match *device {
        InputDeviceType::Desk => BACK_KEY.to_string(),
        InputDeviceType::Pad => gamepad_label(BACK_PAD_BUTTON),
    };
    for button in &q_back {
        for node in q_children.iter_descendants(button) {
            if let Ok(mut text) = q_label.get_mut(node) {
                if text.0 != key {
                    text.0.clone_from(&key);
                }
            }
        }
    }
}

/// A menu button that pops the panel it sits on: [`button`] in the
/// [`ButtonCueType::Back`] voice, and a trailing [`BACK_KEY`] chip because
/// Escape runs it. A button that pops but is not the one Escape runs is a plain
/// [`button`] with [`ButtonCueType::Back`].
pub(crate) fn back_button(text: &str) -> impl Bundle {
    (
        button_variant(text, ButtonVariant::Default, Some(BACK_KEY)),
        ButtonCueType::Back,
        MenuBackChip,
    )
}

/// A details pane's button row. `justify` puts the slots at the pane's start
/// (a list's details) or its end (a modal's commit row). The row wraps, so a
/// narrow pane drops a button to the next line instead of clipping it.
pub(crate) fn actions_row(name: &'static str, justify: JustifyContent) -> impl Bundle {
    (
        Name::new(name),
        Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: justify,
            column_gap: px(8),
            row_gap: px(8),
            margin: UiRect::top(px(12)),
            ..default()
        },
    )
}

/// A fixed-width slot for one button: a themed button fills its parent, so
/// without the slot it spans the whole pane.
pub(crate) fn button_slot(name: &'static str, width: f32) -> impl Bundle {
    (
        Name::new(name),
        Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        },
    )
}

/// The main action's slot width: Load, Create, the Mods Enable/Disable slot.
pub(crate) const MAIN_SLOT_W: f32 = 180.0;
/// A side action's slot width: Delete, Back, Cancel, the Mods portal slots.
pub(crate) const SIDE_SLOT_W: f32 = 140.0;

/// The label column width of a [`detail_row`], so every value in a pane
/// starts on one vertical line.
pub(crate) const DETAIL_LABEL_W: f32 = 110.0;

/// One `LABEL  value` line of a details pane: an uppercase label in a fixed
/// column beside its value.
pub(crate) fn detail_row(
    name: &'static str,
    label: &str,
    value: String,
    color: UiColor,
) -> impl Bundle {
    let label = label.to_uppercase();
    (
        Name::new(name),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            min_height: px(24),
            ..default()
        },
        children![
            (
                UiText,
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                // One line: a two-word label measured as wrapped lifts
                // itself above its value.
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
                Node {
                    width: px(DETAIL_LABEL_W),
                    flex_shrink: 0.0,
                    ..default()
                },
            ),
            (
                UiText,
                Text::new(value),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(color),
            ),
        ],
    )
}
