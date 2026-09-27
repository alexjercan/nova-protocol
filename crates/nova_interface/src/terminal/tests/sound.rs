//! Command modal cues, the ambient bed and the SND mute.

use super::*;

#[test]
fn nova_os_sound_cues_fire_on_terminal_events() {
    let mut app = nova_os_sound_app();

    // Open: the power-up sweep plays and the ambient bed spawns.
    open_commands(&mut app);
    assert!(
        fired(&app, UiSfx::NovaOsPowerUp),
        "opening the modal plays the power-up sweep"
    );
    assert_eq!(bed_count(&mut app), 1, "the ambient bed spawns on open");

    // A keystroke plays the (throttled) typing click.
    clear_capture(&mut app);
    set_prompt(&mut app, "");
    press_text(&mut app, "h");
    assert!(fired(&app, UiSfx::NovaOsKey), "typing plays the key click");

    // A valid command: the enter thunk plus the confirmation beep.
    clear_capture(&mut app);
    set_prompt(&mut app, "help");
    press_enter(&mut app);
    assert!(
        fired(&app, UiSfx::NovaOsEnter),
        "submitting plays the enter thunk"
    );
    assert!(
        fired(&app, UiSfx::NovaOsOk),
        "a valid command plays the ok beep"
    );

    // An unknown command: the error buzz.
    clear_capture(&mut app);
    set_prompt(&mut app, "zzz");
    press_enter(&mut app);
    assert!(
        fired(&app, UiSfx::NovaOsError),
        "an unknown command plays the error buzz"
    );

    // Requesting a close plays the power-down sweep.
    clear_capture(&mut app);
    app.world_mut()
        .resource_mut::<NovaOsCloseTransition>()
        .closing = true;
    app.update();
    assert!(
        fired(&app, UiSfx::NovaOsPowerDown),
        "requesting a close plays the power-down sweep"
    );
}

/// The interface opens and closes with the pause overlay's blip, once per
/// accepted gesture. TAB and Escape pressed together close it in one
/// transition, so they play one blip, not two.
#[test]
fn interface_open_and_close_play_one_toggle_blip() {
    let mut app = nova_os_sound_app();
    let toggles = |app: &App| {
        app.world()
            .resource::<SoundCapture>()
            .0
            .iter()
            .filter(|cue| **cue == UiSfx::UiToggle)
            .count()
    };
    // Press then release, like `press_tab`: the rig has no `InputPlugin` to
    // clear the edge, and the state applies on the second update.
    let press = |app: &mut App, keys: &[KeyCode]| {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in keys {
            input.press(*key);
        }
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release_all();
        input.clear();
        app.update();
    };

    let gestures: [(&str, PauseStates, &[KeyCode], PauseStates); 4] = [
        (
            "Tab open",
            PauseStates::Unpaused,
            &[KeyCode::Tab],
            PauseStates::Interface,
        ),
        (
            "Tab close",
            PauseStates::Interface,
            &[KeyCode::Tab],
            PauseStates::Unpaused,
        ),
        (
            "Escape close",
            PauseStates::Interface,
            &[KeyCode::Escape],
            PauseStates::Unpaused,
        ),
        (
            "Tab and Escape close",
            PauseStates::Interface,
            &[KeyCode::Tab, KeyCode::Escape],
            PauseStates::Unpaused,
        ),
    ];
    for (gesture, from, keys, to) in gestures {
        if pause_state(&app) != from {
            press(&mut app, &[KeyCode::Tab]);
        }
        assert_eq!(pause_state(&app), from, "{gesture} starts in {from:?}");
        clear_capture(&mut app);
        press(&mut app, keys);
        assert_eq!(pause_state(&app), to, "{gesture} reaches {to:?}");
        assert_eq!(toggles(&app), 1, "{gesture} plays one blip");
    }

    // The command modal over flight and over the interface keeps its own power
    // sweep and never blips.
    clear_capture(&mut app);
    open_commands(&mut app);
    assert!(
        fired(&app, UiSfx::NovaOsPowerUp),
        "the modal powers up over flight"
    );
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Unpaused);
    app.update();
    press(&mut app, &[KeyCode::Tab]);
    assert_eq!(pause_state(&app), PauseStates::Interface);
    clear_capture(&mut app);
    open_commands(&mut app);
    assert!(
        fired(&app, UiSfx::NovaOsPowerUp),
        "the modal powers up over the interface"
    );
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    app.update();
    assert_eq!(pause_state(&app), PauseStates::Interface);
    assert_eq!(toggles(&app), 0, "the command modal never blips");
    press(&mut app, &[KeyCode::Escape]);
    assert_eq!(pause_state(&app), PauseStates::Unpaused);

    // A refused Tab plays nothing: paused, then with no ship on the field.
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Paused);
    app.update();
    clear_capture(&mut app);
    press(&mut app, &[KeyCode::Tab]);
    assert_eq!(
        pause_state(&app),
        PauseStates::Paused,
        "Tab is inert while paused"
    );
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Unpaused);
    app.update();
    for ship in app
        .world_mut()
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(app.world())
        .collect::<Vec<_>>()
    {
        app.world_mut().despawn(ship);
    }
    press(&mut app, &[KeyCode::Tab]);
    assert_eq!(
        pause_state(&app),
        PauseStates::Unpaused,
        "Tab is inert with no ship"
    );
    assert_eq!(toggles(&app), 0, "a refused Tab never blips");
}

#[test]
fn nova_os_ambient_bed_tracks_the_modal_state() {
    let mut app = nova_os_sound_app();
    assert_eq!(bed_count(&mut app), 0, "no bed before the modal opens");

    open_commands(&mut app);
    assert_eq!(bed_count(&mut app), 1, "one bed while the modal is open");

    // Leaving the modal despawns the bed. (The freeze loop-pause exemption
    // is structural, not exercised here: the engine's `pause_world_voices`
    // skips every Interface voice, and the bed is one. Asserting the sink stays
    // playing would need an audio device.)
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Unpaused);
    app.update();
    assert_eq!(
        bed_count(&mut app),
        0,
        "the bed despawns when the modal closes"
    );
}

#[test]
fn nova_os_snd_off_silences_cues() {
    let mut app = nova_os_sound_app();
    app.world_mut()
        .resource_mut::<NovaOsMonitorSettings>()
        .sound_enabled = false;

    // Open with SND off: no power-up cue (the bed still spawns, but silent -
    // apply_nova_os_bed_volume drives it to 0).
    open_commands(&mut app);
    assert!(
        !fired(&app, UiSfx::NovaOsPowerUp),
        "SND off silences the power-up sweep"
    );

    // Typing and submitting are silent too.
    set_prompt(&mut app, "help");
    press_enter(&mut app);
    assert!(
        app.world().resource::<SoundCapture>().0.is_empty(),
        "SND off silences every terminal cue, got {:?}",
        app.world().resource::<SoundCapture>().0
    );
}

#[test]
fn nova_os_bed_gain_follows_the_snd_toggle() {
    // The bed's own level. The interface bus, the master and the harness mute
    // are the engine's, applied on top of this.
    assert_eq!(nova_os_bed_gain(true), NOVA_OS_BED_VOLUME);
    assert_eq!(nova_os_bed_gain(false), 0.0, "SND off is dead silent");
}
