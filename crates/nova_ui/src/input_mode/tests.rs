use bevy::ecs::schedule::SystemCondition;

use super::*;

#[derive(Resource, Default)]
struct Ran(bool);

/// Run one system gated on `condition` in a world holding `mode`, and say
/// whether it answered.
fn ran<M>(mode: Option<InputMode>, condition: impl SystemCondition<M>) -> bool {
    let mut app = App::new();
    if let Some(mode) = mode {
        app.insert_resource(mode);
    }
    app.init_resource::<Ran>();
    app.add_systems(
        Update,
        (|mut ran: ResMut<Ran>| ran.0 = true).run_if(condition),
    );
    app.update();
    app.world().resource::<Ran>().0
}

/// An app carrying the arbiter and nothing else.
fn arbiter() -> App {
    let mut app = App::new();
    build(&mut app);
    app
}

#[test]
fn the_most_exclusive_claim_of_the_frame_owns_the_keyboard() {
    let mut app = arbiter();
    app.world_mut()
        .write_message(ClaimKeyboard(InputMode::Browse));
    app.world_mut()
        .write_message(ClaimKeyboard(InputMode::Bind));
    app.world_mut()
        .write_message(ClaimKeyboard(InputMode::Insert));
    app.update();

    assert_eq!(
        *app.world().resource::<InputMode>(),
        InputMode::Bind,
        "three owners asked at once and the least interruptible one has to win"
    );
}

#[test]
fn the_keyboard_goes_back_to_normal_the_frame_nobody_claims_it() {
    let mut app = arbiter();
    app.world_mut()
        .write_message(ClaimKeyboard(InputMode::Browse));
    app.update();
    assert_eq!(*app.world().resource::<InputMode>(), InputMode::Browse);

    app.update();

    assert_eq!(
        *app.world().resource::<InputMode>(),
        InputMode::Normal,
        "a claim is per frame, so a mode ends by going quiet"
    );
}

#[test]
fn a_focused_field_claims_insert() {
    let mut app = arbiter();
    app.world_mut().spawn(TextFieldFocused::at_end("beacon"));

    app.update();

    assert_eq!(*app.world().resource::<InputMode>(), InputMode::Insert);
}

#[test]
fn a_verb_answers_in_normal_alone() {
    for mode in [InputMode::Browse, InputMode::Insert, InputMode::Bind] {
        assert!(
            !ran(Some(mode), in_input_mode(InputMode::Normal)),
            "a verb fired under {mode:?}, which is the whole defect the modes exist to make \
             unreachable"
        );
    }
    assert!(ran(
        Some(InputMode::Normal),
        in_input_mode(InputMode::Normal)
    ));
}

/// R2.8: a save is not a key the gallery reads, and a gallery that covers the
/// whole screen also covers the line a refusal would be written on - so gating
/// Ctrl+S on Normal alone spent it in silence.
#[test]
fn a_verb_no_mode_reads_answers_under_the_quieter_ones() {
    for mode in [InputMode::Normal, InputMode::Browse] {
        assert!(
            ran(Some(mode), in_input_mode_at_most(InputMode::Browse)),
            "{mode:?} takes no key this verb wants"
        );
    }
    for mode in [InputMode::Insert, InputMode::Bind] {
        assert!(
            !ran(Some(mode), in_input_mode_at_most(InputMode::Browse)),
            "{mode:?} is typing or capturing, and both are entitled to the chord"
        );
    }
}

#[test]
fn an_owner_answers_in_its_own_mode_and_in_normal() {
    assert!(
        ran(Some(InputMode::Normal), owns_or_enters(InputMode::Browse)),
        "the key that opens the gallery is pressed before the gallery owns anything"
    );
    assert!(ran(
        Some(InputMode::Browse),
        owns_or_enters(InputMode::Browse)
    ));
    assert!(
        !ran(Some(InputMode::Insert), owns_or_enters(InputMode::Browse)),
        "and a field beats it, because Insert is the more exclusive of the two"
    );
    assert!(!ran(
        Some(InputMode::Bind),
        owns_or_enters(InputMode::Browse)
    ));
    assert!(
        ran(Some(InputMode::Bind), owns_or_enters(InputMode::Bind)),
        "while the mode's own owner keeps answering under it"
    );
}

#[test]
fn an_app_with_no_arbiter_suppresses_nothing() {
    assert!(ran(None, in_input_mode(InputMode::Normal)));
    assert!(ran(None, in_input_mode(InputMode::Bind)));
    assert!(ran(None, owns_or_enters(InputMode::Browse)));
}
