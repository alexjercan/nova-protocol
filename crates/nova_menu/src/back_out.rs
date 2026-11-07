//! Escape on the main menu: back out of the open screen, one level at a time.
//!
//! Every screen on the menu card has a Back button, and Escape answers for it.
//! The answer is the SAME body the button runs, through the shared plain
//! functions beside each button's observer, so a screen cannot have one way out
//! by key and another by pointer. In play the pause overlay answers Escape
//! instead (`toggle_pause`).

use bevy::prelude::*;
use nova_assets::prelude::ReloadContent;
use nova_gameplay::prelude::*;
use nova_ui::input_mode::prelude::InputMode;

#[cfg(not(target_arch = "wasm32"))]
use crate::load_screen::WorldDeleteStep;
use crate::{
    mods::{close_mods, ModsPanel},
    portal::UninstallStep,
    settings::{PendingRebind, ResetBindingsStep},
    widgets::{close_screen, MenuScreen, BACK_PAD_BUTTON},
    world_setup::{back_out_of_new_game, StepNodes, WorldSetupOverlay},
};

/// What a destructive button's armed Confirm looks like to a back-out: the
/// screen is not closed by the press that cancels it.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArmedConfirms<'w> {
    #[cfg(not(target_arch = "wasm32"))]
    delete: ResMut<'w, WorldDeleteStep>,
    reset: ResMut<'w, ResetBindingsStep>,
    uninstall: ResMut<'w, UninstallStep>,
}

impl ArmedConfirms<'_> {
    /// Cancel the first armed confirmation, and say whether there was one.
    pub(crate) fn cancel(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(*self.delete, WorldDeleteStep::Confirm(_)) {
            *self.delete = WorldDeleteStep::Idle;
            return true;
        }
        if *self.reset == ResetBindingsStep::Confirm {
            *self.reset = ResetBindingsStep::Idle;
            return true;
        }
        if *self.uninstall != UninstallStep::Idle {
            *self.uninstall = UninstallStep::Idle;
            return true;
        }
        false
    }
}

/// Escape, or the pad's East button, backs out of the main menu one level.
///
/// It does exactly one of these, first match wins:
/// 1. A rebind chip is armed: nothing. The chip owns that Escape.
/// 2. A text field has the keyboard: nothing. The field owns that Escape.
/// 3. A destructive action waits on its Confirm: cancel it.
/// 4. The New Game modal is open: its Back on the role step, else its Cancel.
/// 5. A screen is shown over the menu card: its Back.
/// 6. Else nothing. Escape on the bare menu must not quit.
///
/// Runs before `apply_settings_rebind`, which drops the armed chip on the same
/// Escape: rung 1 has to read the chip as the press found it.
pub(crate) fn back_out_of_the_menu_screen(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    rebind: Res<PendingRebind>,
    input_mode: Res<InputMode>,
    mut confirms: ArmedConfirms,
    overlays: Query<Entity, With<WorldSetupOverlay>>,
    mut nodes: StepNodes,
    mut screens: Query<(&mut Visibility, Has<ModsPanel>), With<MenuScreen>>,
    mut reload: MessageWriter<ReloadContent>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let pad = gamepads
        .iter()
        .any(|pad| pad.digital().just_pressed(BACK_PAD_BUTTON));
    if !(keys.just_pressed(KeyCode::Escape) || pad) {
        return;
    }
    if rebind.is_armed() || *input_mode >= InputMode::Insert {
        return;
    }
    if confirms.cancel() {
        play_menu_back(&mut commands, bank.as_deref());
        return;
    }
    if back_out_of_new_game(&mut commands, &overlays, &mut nodes) {
        play_menu_back(&mut commands, bank.as_deref());
        return;
    }
    for (mut visibility, is_mods) in &mut screens {
        if *visibility == Visibility::Hidden {
            continue;
        }
        if is_mods {
            close_mods(&mut visibility, &mut reload);
        } else {
            close_screen(&mut visibility);
        }
        play_menu_back(&mut commands, bank.as_deref());
        return;
    }
}
