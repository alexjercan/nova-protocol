//! NOVA OS terminal cues, the ambient bed and the SND mute.

use super::*;

#[test]
fn nova_os_sound_cues_fire_on_terminal_events() {
    let mut app = nova_os_sound_app();

    // Open: the power-up sweep plays and the ambient bed spawns.
    open_nova_os(&mut app);
    assert!(
        fired(&app, UiSfx::NovaOsPowerUp),
        "opening the computer plays the power-up sweep"
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

#[test]
fn nova_os_ambient_bed_tracks_nova_os_state() {
    let mut app = nova_os_sound_app();
    assert_eq!(bed_count(&mut app), 0, "no bed before the computer opens");

    open_nova_os(&mut app);
    assert_eq!(bed_count(&mut app), 1, "one bed while the computer is open");

    // Leaving the NOVA OS despawns the bed. (The freeze loop-pause exemption
    // is structural, not exercised here: `audio::pause_loops` queries only
    // ThrusterLoopSfx/RcsLoopSfx, so NovaOsBedSfx is never paused - see the
    // task note. Asserting the sink stays playing would need an audio
    // device.)
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Unpaused);
    app.update();
    assert_eq!(
        bed_count(&mut app),
        0,
        "the bed despawns when the computer closes"
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
    open_nova_os(&mut app);
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
fn nova_os_bed_gain_respects_snd_and_master() {
    // The bed's volume logic (the sink write needs an audio device, so the
    // gain is factored out pure). SND on at full master -> the base volume.
    assert_eq!(nova_os_bed_gain(true, 1.0), NOVA_OS_BED_VOLUME);
    // SND off -> dead silent, whatever the master.
    assert_eq!(nova_os_bed_gain(false, 1.0), 0.0);
    // A zero master output gain (volume 0 OR a HarnessMute'd run) -> silent.
    assert_eq!(nova_os_bed_gain(true, 0.0), 0.0);
    // Half master scales the hum.
    assert_eq!(nova_os_bed_gain(true, 0.5), NOVA_OS_BED_VOLUME * 0.5);
}
