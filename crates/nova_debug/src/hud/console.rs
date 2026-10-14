//! The Cheats and Command sections of the Nova Debug window.
//!
//! Both drive the game's own command shell, [`CommandTerminal`], so the window
//! has no parser, no dispatcher and no cheat code of its own. The prompt edits
//! the CRT's prompt and Enter submits it; the dispatcher (`nova_console`)
//! runs the command on the next `Update` and writes its rows to the shared
//! scrollback, refusals included. A cheat button builds the command line with
//! the player ship's id and submits it the same way, so an unarmed run gets
//! the dispatcher's refusal and `Arm cheats` is `cheats enable`.

use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::input::EguiWantsInput,
    egui::{
        self,
        text::{CCursor, CCursorRange},
        text_edit::TextEditState,
    },
};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::PlayerSpaceshipMarker;
use nova_interface::nova_command::prelude::{CommandTerminal, TerminalRowKind};
use nova_ui::prelude::{ClaimKeyboard, InputMode};

use super::{close_debug_hud, DebugHud};

/// What an app without the shell shows in place of both sections.
const NO_SHELL: &str = "this app has no command shell";

/// While the window is open and an egui widget has the keyboard (the prompt,
/// the item field, an inspector field), keys are text: claim
/// [`InputMode::Insert`] so `:`, TAB-pane keys and other verbs stay quiet.
///
/// Reads the previous frame's [`EguiWantsInput`]: `bevy_egui` writes it in
/// `PostUpdate`, after the window drew.
pub(crate) fn claim_keyboard_for_debug_hud(
    hud: Res<DebugHud>,
    wants: Option<Res<EguiWantsInput>>,
    mut claims: MessageWriter<ClaimKeyboard>,
) {
    if hud.open && wants.is_some_and(|wants| wants.wants_keyboard_input()) {
        claims.write(ClaimKeyboard(InputMode::Insert));
    }
}

/// Submit the shell's current prompt line, as Enter at the CRT does. `close`
/// closes this window here, so the CRT never reads a close it did not issue.
fn submit_prompt(world: &mut World) {
    let Some(mut terminal) = world.get_resource_mut::<CommandTerminal>() else {
        return;
    };
    terminal.submit();
    if terminal.take_pending_close() {
        close_debug_hud(world);
    }
}

/// Submit `line` through the shell and put back what was typed at the prompt.
pub(crate) fn submit_line(world: &mut World, line: &str) {
    let Some(mut terminal) = world.get_resource_mut::<CommandTerminal>() else {
        return;
    };
    let typed = terminal.prompt().to_owned();
    terminal.reset_prompt();
    terminal.insert_text(line);
    submit_prompt(world);
    if let Some(mut terminal) = world.get_resource_mut::<CommandTerminal>() {
        terminal.insert_text(&typed);
    }
}

/// The Cheats section: arming, then one button per cheat on the player ship.
pub(crate) fn cheats_ui(world: &mut World, ui: &mut egui::Ui, item: &mut String) {
    let Some(terminal) = world.get_resource::<CommandTerminal>() else {
        ui.colored_label(ui.visuals().error_fg_color, NO_SHELL);
        return;
    };
    let last = terminal.scrollback().last().cloned();
    let mut line = None;
    if ui.button("Arm cheats").clicked() {
        line = Some("cheats enable".to_owned());
    }
    let player = world
        .query_filtered::<&EntityId, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .next()
        .map(|id| id.0.clone());
    match player {
        None => {
            ui.label("no player ship");
        }
        Some(id) => {
            ui.label(format!("player ship: {id}"));
            ui.horizontal(|ui| {
                ui.label("Ammo");
                if ui.button("infinite on").clicked() {
                    line = Some(format!("ammo infinite {id} on"));
                }
                if ui.button("infinite off").clicked() {
                    line = Some(format!("ammo infinite {id} off"));
                }
                if ui.button("refill").clicked() {
                    line = Some(format!("ammo refill {id}"));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Credits");
                if ui.button("give 10000").clicked() {
                    line = Some(format!("credits give {id} 10000"));
                }
                if ui.button("set 0").clicked() {
                    line = Some(format!("credits set {id} 0"));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Item");
                ui.add(
                    egui::TextEdit::singleline(item)
                        .hint_text("<item> <n>")
                        .desired_width(140.0),
                );
                if ui.button("give").clicked() {
                    line = Some(format!("item give {id} {}", item.trim()));
                }
            });
        }
    }
    if let Some(last) = last {
        ui.label(row_text(ui, last.kind, &last.text));
    }
    if let Some(line) = line {
        submit_line(world, &line);
    }
}

/// The Command section: the shared scrollback and a prompt over the shell's.
///
/// TAB completes, Up and Down walk the history and Enter submits, as at the
/// CRT. The keys are taken from egui before the field sees them, so TAB does
/// not move the focus.
pub(crate) fn command_ui(world: &mut World, ui: &mut egui::Ui) {
    let Some(terminal) = world.get_resource::<CommandTerminal>() else {
        ui.colored_label(ui.visuals().error_fg_color, NO_SHELL);
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt("nova_debug_scrollback")
        .max_height(220.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for row in terminal.scrollback() {
                ui.label(row_text(ui, row.kind, &row.text));
            }
        });

    let id = egui::Id::new("nova_debug_prompt");
    let (tab, up, down) = if ui.memory(|memory| memory.has_focus(id)) {
        ui.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::NONE, egui::Key::Tab),
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            )
        })
    } else {
        (false, false, false)
    };
    let prefix = terminal.prompt_prefix();
    let mut text = terminal.prompt().to_owned();
    let output = ui
        .horizontal(|ui| {
            ui.monospace(prefix);
            egui::TextEdit::singleline(&mut text)
                .id(id)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .show(ui)
        })
        .inner;
    let enter =
        output.response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

    let mut terminal = world.resource_mut::<CommandTerminal>();
    if text != terminal.prompt() {
        terminal.reset_prompt();
        terminal.insert_text(&text);
    }
    if tab {
        terminal.complete();
    }
    if up {
        terminal.history_previous();
    }
    if down {
        terminal.history_next();
    }
    if tab || up || down {
        // The shell rewrote the line; put egui's caret at its end, where the
        // shell's caret is.
        let end = CCursor::new(terminal.prompt().chars().count());
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::one(end)));
        state.store(ui.ctx(), id);
    }
    if let Some(hint) = terminal.completion_hint() {
        ui.weak(hint);
    }
    if enter {
        submit_prompt(world);
        output.response.request_focus();
    }
}

/// A scrollback row in the CRT's colour for its kind.
fn row_text(ui: &egui::Ui, kind: TerminalRowKind, text: &str) -> egui::RichText {
    let visuals = ui.visuals();
    let color = match kind {
        TerminalRowKind::Input => visuals.strong_text_color(),
        TerminalRowKind::Output => visuals.text_color(),
        TerminalRowKind::Dim => visuals.weak_text_color(),
        TerminalRowKind::Info => egui::Color32::from_rgb(120, 200, 255),
        TerminalRowKind::Warn => visuals.warn_fg_color,
        TerminalRowKind::Error => visuals.error_fg_color,
    };
    egui::RichText::new(text).monospace().color(color)
}

#[cfg(test)]
mod tests {
    use bevy::{
        input::{
            keyboard::{Key, KeyboardInput},
            ButtonState, InputPlugin,
        },
        state::app::StatesPlugin,
    };
    use bevy_inspector_egui::bevy_egui::{input::write_egui_wants_input_system, EguiContext};
    use nova_console::prelude::NovaConsolePlugin;
    use nova_gameplay::{
        prelude::{ClockFreeze, RunCheats, ShipCredits, SpaceshipRootMarker},
        GameStates, PauseStates,
    };
    use nova_ui::NovaUiPlugin;

    use super::*;
    use crate::hud::DebugHudPlugin;

    /// The game's dispatcher over one player ship, `player`, holding 410 cr.
    fn shell(armed: bool) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(GameStates::Playing);
        app.init_state::<PauseStates>();
        app.init_resource::<CommandTerminal>();
        let mut cheats = RunCheats::default();
        if armed {
            cheats.arm();
        }
        app.insert_resource(cheats);
        app.add_plugins(NovaConsolePlugin);
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId("player".to_owned()),
                ShipCredits(410),
            ))
            .id();
        app.update();
        (app, ship)
    }

    fn scrollback(app: &App) -> Vec<String> {
        let terminal = app.world().resource::<CommandTerminal>();
        terminal
            .scrollback()
            .iter()
            .map(|row| row.text.clone())
            .collect()
    }

    #[test]
    fn a_command_submitted_from_the_window_runs_and_answers_in_the_scrollback() {
        let (mut app, ship) = shell(true);

        submit_line(app.world_mut(), "credits give player 100");
        app.update();

        assert_eq!(
            app.world().get::<ShipCredits>(ship),
            Some(&ShipCredits(510))
        );
        let rows = scrollback(&app);
        assert!(
            rows.iter()
                .any(|row| row.contains("credits 410 cr -> 510 cr")),
            "the dispatcher's answer is in the shared scrollback: {rows:?}"
        );
    }

    #[test]
    fn an_unarmed_cheat_shows_the_dispatchers_refusal() {
        let (mut app, ship) = shell(false);

        submit_line(app.world_mut(), "credits give player 10000");
        app.update();

        assert_eq!(
            app.world().get::<ShipCredits>(ship),
            Some(&ShipCredits(410))
        );
        let rows = scrollback(&app);
        assert!(
            rows.iter().any(|row| row.contains("cheats are not armed")),
            "the refusal row is on the screen: {rows:?}"
        );
    }

    /// A stand-in for `nova_menu`'s `open_command_shell` under that system's
    /// own gate: it reads every frame, and `:` opens the CRT in
    /// [`InputMode::Normal`] only.
    fn open_shell_on_colon(
        mut keyboard: MessageReader<KeyboardInput>,
        mode: Res<InputMode>,
        mut next: ResMut<NextState<PauseStates>>,
    ) {
        let typed_colon = keyboard
            .read()
            .any(|event| event.state == ButtonState::Pressed && event.text.as_deref() == Some(":"));
        if typed_colon && *mode == InputMode::Normal {
            next.set(PauseStates::Commands);
        }
    }

    fn type_colon(app: &mut App) {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Semicolon,
            logical_key: Key::Character(":".into()),
            state: ButtonState::Pressed,
            text: Some(":".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    #[test]
    fn a_colon_typed_into_the_window_does_not_open_the_command_shell() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin, NovaUiPlugin));
        app.insert_state(GameStates::Playing);
        app.init_state::<PauseStates>();
        app.init_resource::<ClockFreeze>();
        app.init_resource::<EguiWantsInput>();
        app.add_plugins(DebugHudPlugin);
        app.add_systems(
            PreUpdate,
            write_egui_wants_input_system.before(crate::hud::DebugHudSystems::Claim),
        );
        app.add_systems(Update, open_shell_on_colon);
        // An egui context whose text field has the keyboard, as the prompt
        // does once clicked.
        let mut context = EguiContext::default();
        let mut typed = String::new();
        for _ in 0..2 {
            let _ = context.get_mut().run_ui(egui::RawInput::default(), |ui| {
                ui.text_edit_singleline(&mut typed).request_focus();
            });
        }
        app.world_mut().spawn(context);
        app.world_mut().resource_mut::<DebugHud>().open = true;
        app.update();

        type_colon(&mut app);
        assert_eq!(*app.world().resource::<InputMode>(), InputMode::Insert);
        assert_eq!(
            *app.world().resource::<State<PauseStates>>().get(),
            PauseStates::Debug,
            "the colon is text in the window's field"
        );

        app.world_mut().resource_mut::<DebugHud>().open = false;
        app.update();
        type_colon(&mut app);
        app.update();
        assert_eq!(
            *app.world().resource::<State<PauseStates>>().get(),
            PauseStates::Commands,
            "with the window closed, the colon opens the shell"
        );
    }
}
