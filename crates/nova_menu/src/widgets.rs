//! Shared menu widget primitives: the click-cue marker and the two button
//! constructors every menu surface spawns.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ui::widget::{menu_button, ButtonSpec, ButtonVariant};

/// Marks menu-family buttons (main menu, pause, outcome, mods checkbox) so the
/// press-cue observer plays the click only for them, leaving the editor's
/// `ThemedButton`s silent. Colour comes from the shared nova_ui observers.
#[derive(Component)]
pub(crate) struct MenuSfxButton;

/// Play the menu-button click for any [`MenuSfxButton`] activation. One global observer
/// covers every menu and pause-overlay button - the `button()` helper always carries
/// `MenuSfxButton` - so presses that were visual-only (hover/press colours) now also
/// have a voice. A missing [`SoundBank`] (assets not loaded) is a graceful no-op.
pub(crate) fn on_menu_button_activate(
    activate: On<Activate>,
    q_button: Query<(), With<MenuSfxButton>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if !q_button.contains(activate.entity) {
        return;
    }
    if let Some(bank) = bank {
        commands.play_sfx_volume(bank.get(UiSfx::MenuSelect), MENU_SELECT_VOLUME);
    }
}

/// The main-menu / pause / outcome button: the shared nova_ui [`menu_button`] (40px /
/// 16px `ThemedButton`, coloured by the nova_ui observers + skin reconciler) plus the
/// [`MenuSfxButton`] marker that scopes the click cue to menu-family buttons (the
/// editor's `ThemedButton`s stay silent). One observer path for every button in the
/// game.
pub(crate) fn button(text: &str) -> impl Bundle {
    (menu_button(text), MenuSfxButton)
}

/// A menu button carrying an emphasis variant (primary call-to-action / danger)
/// and an optional trailing key-chip - the shared nova_ui `ThemedButton` at the
/// 40/16 menu sizing, plus the `MenuSfxButton` click cue.
pub(crate) fn button_variant(text: &str, variant: ButtonVariant, key: Option<&str>) -> impl Bundle {
    let mut spec = ButtonSpec::new(text).menu();
    spec.variant = variant;
    if let Some(key) = key {
        spec = spec.key(key);
    }
    (nova_ui::widget::button(spec), MenuSfxButton)
}
