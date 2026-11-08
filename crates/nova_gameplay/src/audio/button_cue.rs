//! The voice of every button: the press cue and the hover tick.
//!
//! One owner, so a new button is heard without opting in. The cue a press plays
//! is a property of the BUTTON, read off its
//! [`ButtonCueType`](nova_ui::widget::ButtonCueType), not of its label:
//! matching on text would put "Back to Main Menu" in the wrong voice the first
//! time someone renames it.
//!
//! This lives here rather than in `nova_ui` because `nova_ui` has no sound bank:
//! the sounds are the one thing it cannot depend on.

use bevy::{
    picking::{
        hover::Hovered,
        pointer::{PointerLocation, PointerPressState},
    },
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button},
};
use nova_ui::widget::{ButtonCueMute, ButtonCueType};

use super::{
    AudioRoute, SfxCommandsExt, SoundBank, UiSfx, EDITOR_DENY_VOLUME, MENU_BACK_VOLUME,
    MENU_FOCUS_VOLUME, MENU_SELECT_VOLUME,
};

/// The button voice's systems: today the hover tick, the one cue that is not
/// an observer.
///
/// Runs in `Update` with no ordering constraint inside that schedule, and the
/// absence is the statement: the tick reads `Hovered`, which bevy's picking
/// writes back in `PreUpdate`, and it raises a voice the engine's
/// `AudioSystems` pass mixes in `PostUpdate`. Both edges are schedule
/// boundaries, so nothing in `Update` can come between them.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ButtonCueSystems;

/// Plays every button's press cue and hover tick. [`NovaAudioPlugin`] adds it;
/// a test rig that hears cues without the audio engine adds it alone.
///
/// [`NovaAudioPlugin`]: super::NovaAudioPlugin
pub struct ButtonCuePlugin;

impl Plugin for ButtonCuePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_button_activate);
        app.add_systems(Update, play_button_focus_cue.in_set(ButtonCueSystems));
    }
}

/// Play the interface click, if the sound bank has loaded. For a gesture with
/// no button under it, and for a [`ButtonCueType::Owned`] handler.
pub fn play_menu_select(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>) {
    if let Some(bank) = bank {
        commands.play_sfx(
            bank.get(UiSfx::MenuSelect),
            AudioRoute::Interface,
            MENU_SELECT_VOLUME,
        );
    }
}

/// Play the panel-pop cue, if the sound bank has loaded. For a back-out key
/// with no button to press.
pub fn play_menu_back(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>) {
    if let Some(bank) = bank {
        commands.play_sfx(
            bank.get(UiSfx::MenuBack),
            AudioRoute::Interface,
            MENU_BACK_VOLUME,
        );
    }
}

/// Play the refusal buzz, if the sound bank has loaded. For a
/// [`ButtonCueType::Owned`] handler whose press was refused, in place of the
/// click.
pub fn play_menu_deny(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>) {
    if let Some(bank) = bank {
        commands.play_sfx(
            bank.get(UiSfx::EditorDeny),
            AudioRoute::Interface,
            EDITOR_DENY_VOLUME,
        );
    }
}

/// Play the press cue of an accepted activation.
///
/// Global, so it runs before the button's own `.observe` handler and a handler
/// that despawns the button cannot lose the cue. The disabled check covers an
/// `Activate` triggered directly: the widget itself never raises one on a
/// disabled button. A missing [`SoundBank`] (assets not loaded) is silence.
fn on_button_activate(
    activate: On<Activate>,
    q_button: Query<
        (Option<&ButtonCueType>, Option<&ButtonCueMute>),
        (With<Button>, Without<InteractionDisabled>),
    >,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok((cue, mute)) = q_button.get(activate.entity) else {
        return;
    };
    if mute.is_some_and(|mute| mute.0) {
        return;
    }
    match cue.copied().unwrap_or_default() {
        ButtonCueType::Select => play_menu_select(&mut commands, bank.as_deref()),
        ButtonCueType::Back => play_menu_back(&mut commands, bank.as_deref()),
        ButtonCueType::Owned => {}
    }
}

/// Tick when the cursor ARRIVES on a button.
///
/// A system on `Changed<Hovered>` rather than an observer, because `Hovered` is
/// written over a live component, so there is no `Add` to hang a cue on. Rising
/// edge only - leaving a button is silent, or a sweep across a column would
/// fire twice per button. One tick a frame however many rise: `Hovered` also
/// lights an ancestor, so a chip inside a row rises with its row.
///
/// Only a free pointer that moved this frame arrives. A row rebuilt or scrolled
/// under a still pointer rises too, and an orbit drag sweeps across every blip
/// in its way, and neither gesture is aimed at a button.
///
/// A DISABLED button is silent. It cannot be pressed, so a tick that says
/// "you are on something" would be a lie, and the greyed paint is already the
/// honest answer. A [`ButtonCueMute`] button is silent because its owner says so.
fn play_button_focus_cue(
    q_hovered: Query<
        (&Hovered, Has<InteractionDisabled>, Option<&ButtonCueMute>),
        (With<Button>, Changed<Hovered>),
    >,
    q_pointer: Query<(Ref<PointerLocation>, &PointerPressState)>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Some(bank) = bank else {
        return;
    };
    if !q_pointer
        .iter()
        .any(|(location, press)| location.is_changed() && !press.is_any_pressed())
    {
        return;
    }
    if q_hovered.iter().any(|(hovered, disabled, mute)| {
        hovered.get() && !disabled && !mute.is_some_and(|mute| mute.0)
    }) {
        commands.play_sfx(
            bank.get(UiSfx::MenuFocus),
            AudioRoute::Interface,
            MENU_FOCUS_VOLUME,
        );
    }
}
