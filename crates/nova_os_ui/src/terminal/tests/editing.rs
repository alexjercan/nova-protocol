//! The prompt as a text field: caret jumps, readline chords, and the boot
//! reveal a deliberate key skips.

use super::*;

/// Home and End reach the ends of the line, and so do their chords. The chords
/// are keyed on the PHYSICAL key because a held Control makes the produced text
/// a control character on some platforms and the letter on others - so the
/// events here carry the control character winit reports on X11.
#[test]
fn the_prompt_answers_home_end_and_their_readline_chords() {
    let mut app = terminal_command_app();
    press_text(&mut app, "map goto");

    press_key(&mut app, KeyCode::Home, Key::Home, None);
    assert_eq!(cursor(&app), 0);
    press_key(&mut app, KeyCode::End, Key::End, None);
    assert_eq!(cursor(&app), "map goto".len());

    hold_control(&mut app);
    chord(&mut app, KeyCode::KeyA, 'a');
    assert_eq!(cursor(&app), 0, "Ctrl+A is Home");
    chord(&mut app, KeyCode::KeyE, 'e');
    assert_eq!(cursor(&app), "map goto".len(), "Ctrl+E is End");
}

/// The two kills, and the rule that holds them together: a chord the prompt
/// does not answer must still not type its letter.
#[test]
fn a_kill_cuts_its_half_and_an_unanswered_chord_types_nothing() {
    let mut app = terminal_command_app();
    press_text(&mut app, "map goto");
    hold_control(&mut app);

    chord(&mut app, KeyCode::KeyU, 'u');
    assert_eq!(prompt(&app), "", "Ctrl+U cuts back to the start");

    release_control(&mut app);
    press_text(&mut app, "map goto");
    press_key(&mut app, KeyCode::Home, Key::Home, None);
    hold_control(&mut app);
    chord(&mut app, KeyCode::KeyK, 'k');
    assert_eq!(prompt(&app), "", "Ctrl+K cuts on to the end");

    release_control(&mut app);
    press_text(&mut app, "log");
    hold_control(&mut app);
    // W is a chord readline has and this prompt does not.
    chord(&mut app, KeyCode::KeyW, 'w');
    assert_eq!(
        prompt(&app),
        "log",
        "an unanswered chord is swallowed, not typed"
    );
}

/// The banner is an animation. A player who knows the command they want
/// finishes it by starting to type - and the key that finished it still lands,
/// so the first letter is not eaten.
#[test]
fn the_first_deliberate_key_finishes_the_boot_reveal_and_still_lands() {
    let mut app = terminal_command_app();
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .begin_boot(nova_os_boot_banner_rows(0, None));
    let queued = app.world().resource::<NovaOsTerminal>().scrollback().len();
    assert_eq!(queued, 0, "the banner starts on an empty screen");

    // A modifier is not an instruction: holding Shift to capitalise a letter
    // must not be what skips the reveal.
    press_key(&mut app, KeyCode::ShiftLeft, Key::Shift, None);
    assert!(
        app.world()
            .resource::<NovaOsTerminal>()
            .has_pending_boot_rows(),
        "a modifier alone leaves the reveal running"
    );

    press_text(&mut app, "l");
    let terminal = app.world().resource::<NovaOsTerminal>();
    assert!(
        !terminal.has_pending_boot_rows(),
        "the letter finished the banner"
    );
    assert!(
        !terminal.scrollback().is_empty(),
        "and the whole banner is on screen"
    );
    assert_eq!(
        terminal.prompt(),
        "l",
        "the letter that finished it still typed"
    );
}

fn prompt(app: &App) -> String {
    app.world()
        .resource::<NovaOsTerminal>()
        .prompt()
        .to_string()
}

fn cursor(app: &App) -> usize {
    app.world().resource::<NovaOsTerminal>().cursor()
}

fn hold_control(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
}

fn release_control(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
}

/// One chord press, spelled the way X11 reports it with Control down: the
/// logical key is the control CHARACTER, and only `key_code` names the letter.
fn chord(app: &mut App, key_code: KeyCode, letter: char) {
    let control_char = char::from((letter as u8) & 0x1f);
    press_key(
        app,
        key_code,
        Key::Character(control_char.to_string().into()),
        None,
    );
}
