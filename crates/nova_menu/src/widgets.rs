//! Shared menu widget primitives: the cue markers and the button constructors
//! every menu surface spawns.
//!
//! Three voices, and which one a button gets is a property of the BUTTON, not
//! of its label: [`MenuSfxButton`] is the click, [`MenuSfxBack`] inverts it for
//! a button that pops a panel, and the hover pass gives the whole family a
//! focus tick. Matching on text would put "Back to Main Menu" in the wrong
//! voice the first time someone renames it.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ui::widget::{menu_button, ButtonSpec, ButtonVariant};

/// Marks menu-family buttons (main menu, pause, outcome, mods checkbox) so the
/// press-cue observer plays the click only for them, leaving the editor's
/// `ThemedButton`s silent. Colour comes from the shared nova_ui observers.
#[derive(Component)]
pub(crate) struct MenuSfxButton;

/// Marks a menu button that POPS the surface it sits on - the "Back" out of a
/// settings panel or a mods list. It plays [`UiSfx::MenuBack`] instead of the
/// click, so entering and leaving a panel are audibly a direction.
///
/// Only a button that RETURNS carries it. "Retry" and "Main Menu" on the
/// outcome banner both leave the screen, but they are commits - the run is
/// over either way - and a commit is a click.
#[derive(Component)]
pub(crate) struct MenuSfxBack;

/// Play a menu button's press cue. One global observer covers every menu and
/// pause-overlay button - the `button()` helper always carries
/// [`MenuSfxButton`] - and [`MenuSfxBack`] selects the inverted voice. A
/// missing [`SoundBank`] (assets not loaded) is a graceful no-op.
pub(crate) fn on_menu_button_activate(
    activate: On<Activate>,
    q_button: Query<Has<MenuSfxBack>, With<MenuSfxButton>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(back) = q_button.get(activate.entity) else {
        return;
    };
    let Some(bank) = bank else {
        return;
    };
    let (key, volume) = if back {
        (UiSfx::MenuBack, MENU_BACK_VOLUME)
    } else {
        (UiSfx::MenuSelect, MENU_SELECT_VOLUME)
    };
    commands.play_sfx(bank.get(key), AudioRoute::Interface, volume);
}

/// The menu's own CUE systems - today the focus tick, which is the whole of
/// the menu's voice that is not an observer.
///
/// Runs in `Update` with no ordering constraint inside that schedule, and the
/// absence is the statement: the tick reads `Hovered`, which bevy's picking
/// writes back in `PreUpdate`, and it raises a voice the engine's
/// `AudioSystems` pass mixes in `PostUpdate`. Both edges are schedule
/// boundaries, so nothing in `Update` can come between them.
///
/// It exists so the menu's voice is one nameable thing: a second cue joins it
/// rather than being added loose, an outside plugin can order against it, and
/// a run condition applies to the whole voice at once.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuCueSystems;

/// Tick when the cursor ARRIVES on a menu button.
///
/// A system on `Changed<Hovered>` rather than an observer, because `Hovered` is
/// a live component every button carries from birth: it is written in place,
/// so there is no `Add` to hang a cue on. Rising edge only - leaving a button
/// is silent, or a sweep across a column would fire twice per button.
///
/// A DISABLED button is silent. It cannot be pressed, so a tick that says
/// "you are on something" would be a lie, and the greyed paint is already the
/// honest answer.
pub(crate) fn play_menu_focus_cue(
    q_hovered: Query<
        (&Hovered, Has<bevy::ui::InteractionDisabled>),
        (With<MenuSfxButton>, Changed<Hovered>),
    >,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Some(bank) = bank else {
        return;
    };
    for (hovered, disabled) in &q_hovered {
        if !hovered.get() || disabled {
            continue;
        }
        commands.play_sfx(
            bank.get(UiSfx::MenuFocus),
            AudioRoute::Interface,
            MENU_FOCUS_VOLUME,
        );
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

/// A menu button that pops the panel it sits on: [`button`] plus the
/// [`MenuSfxBack`] marker that swaps its voice.
pub(crate) fn back_button(text: &str) -> impl Bundle {
    (button(text), MenuSfxBack)
}
