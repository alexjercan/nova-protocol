//! The chin hardware: BRIGHT/SCAN knobs, the SND bulb and the PWR LED.

use super::*;

#[test]
fn nova_os_chin_knobs_cycle_detents() {
    let mut app = chin_controls_app();

    // A sampling surface + material gives `animate_nova_os_crt` a uniform
    // target (the render-capable RTT path is not built headless).
    let handle = app
        .world_mut()
        .resource_mut::<Assets<NovaOsCrtMaterial>>()
        .add(NovaOsCrtMaterial::default());
    app.world_mut().spawn((
        NovaOsSamplingSurfaceMarker,
        MaterialNode(handle.clone()),
        ComputedNode {
            size: Vec2::new(800.0, 600.0),
            ..default()
        },
    ));

    let bright = knob_button(&mut app, NovaOsKnob::Bright);
    assert_eq!(
        app.world()
            .resource::<NovaOsMonitorSettings>()
            .bright_detent,
        NOVA_OS_BRIGHT_DEFAULT_DETENT,
        "BRIGHT boots at the neutral detent"
    );

    // One click advances the detent, rotates the dial and drives the
    // brightness uniform.
    app.world_mut().trigger(Activate { entity: bright });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NovaOsMonitorSettings>()
            .bright_detent,
        2,
        "a BRIGHT click advances one detent"
    );
    assert!(
        (dial_rotation(&mut app, NovaOsKnob::Bright) - NOVA_OS_KNOB_ANGLES[2].to_radians()).abs()
            < 1e-3,
        "the dial pointer rotates to the new detent angle"
    );
    let brightness = app
        .world()
        .resource::<Assets<NovaOsCrtMaterial>>()
        .get(&handle)
        .unwrap()
        .data
        .brightness;
    assert_eq!(
        brightness, NOVA_OS_BRIGHT_DETENTS[2],
        "the CRT brightness uniform follows the BRIGHT detent"
    );

    // Four detents wrap back to the start.
    for _ in 0..3 {
        app.world_mut().trigger(Activate { entity: bright });
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<NovaOsMonitorSettings>()
            .bright_detent,
        NOVA_OS_BRIGHT_DEFAULT_DETENT,
        "the 4 BRIGHT detents cycle and wrap"
    );

    // SCAN cycles independently and drives the scanline uniform.
    let scan = knob_button(&mut app, NovaOsKnob::Scan);
    app.world_mut().trigger(Activate { entity: scan });
    app.update();
    assert_eq!(
        app.world().resource::<NovaOsMonitorSettings>().scan_detent,
        3,
        "a SCAN click advances one detent, independent of BRIGHT"
    );
    let scanline = app
        .world()
        .resource::<Assets<NovaOsCrtMaterial>>()
        .get(&handle)
        .unwrap()
        .data
        .scanline_strength;
    assert_eq!(
        scanline, NOVA_OS_SCAN_DETENTS[3],
        "the CRT scanline uniform follows the SCAN detent"
    );
}

#[test]
fn nova_os_snd_toggles_sound_resource() {
    let mut app = chin_controls_app();
    assert!(
        app.world()
            .resource::<NovaOsMonitorSettings>()
            .sound_enabled,
        "the monitor speaker defaults ON"
    );
    let snd = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsSoundButtonMarker>>()
        .iter(app.world())
        .next()
        .expect("the SND button spawned");

    // Armed: the bulb is lit phosphor and the label is the fixed legend.
    assert_eq!(
        bulb_color::<NovaOsSoundIndicatorMarker>(&mut app),
        NOVA_OS_PHOSPHOR,
        "the SND bulb is lit while the monitor is armed"
    );
    assert!(
        all_texts(&mut app).iter().any(|text| text == "SND"),
        "the SND label is a fixed legend"
    );
    assert!(
        all_texts(&mut app).iter().all(|text| text != "SND ON"),
        "the SND label no longer swaps to an ON/OFF state string"
    );

    app.world_mut().trigger(Activate { entity: snd });
    app.update();
    assert!(
        !app.world()
            .resource::<NovaOsMonitorSettings>()
            .sound_enabled,
        "a SND click mutes the monitor"
    );
    // Muted: the state now reads off the bulb going dark, not a label swap.
    assert_eq!(
        bulb_color::<NovaOsSoundIndicatorMarker>(&mut app),
        NOVA_OS_BULB_OFF,
        "the SND bulb goes dark to report the muted state"
    );
    assert!(
        all_texts(&mut app).iter().any(|text| text == "SND"),
        "the SND label stays the fixed legend when muted"
    );

    app.world_mut().trigger(Activate { entity: snd });
    app.update();
    assert!(
        app.world()
            .resource::<NovaOsMonitorSettings>()
            .sound_enabled,
        "a second SND click re-arms the monitor"
    );
    assert_eq!(
        bulb_color::<NovaOsSoundIndicatorMarker>(&mut app),
        NOVA_OS_PHOSPHOR,
        "re-arming re-lights the SND bulb"
    );
}

#[test]
fn nova_os_pwr_drives_close_transition() {
    let mut app = chin_controls_app();
    assert!(
        !app.world().resource::<NovaOsCloseTransition>().closing,
        "the computer is open, not closing"
    );
    let pwr = app
        .world_mut()
        .query_filtered::<Entity, With<NovaOsPowerButtonMarker>>()
        .iter(app.world())
        .next()
        .expect("the PWR button spawned");

    app.world_mut().trigger(Activate { entity: pwr });
    app.update();
    assert!(
        app.world().resource::<NovaOsCloseTransition>().closing,
        "PWR drives the existing animated close"
    );
}

#[test]
fn nova_os_pwr_led_flashes_orange_while_closing() {
    let mut app = chin_controls_app();

    // Powered and idle: the LED sits at lit phosphor green.
    app.world_mut()
        .run_system_once(drive_nova_os_power_led)
        .unwrap();
    assert_eq!(
        bulb_color::<NovaOsPowerLedMarker>(&mut app),
        NOVA_OS_PHOSPHOR,
        "the PWR LED is green while the monitor is powered"
    );

    // Powering down (the state PWR sets): the LED flashes orange before the
    // raster collapse finishes the close.
    app.world_mut()
        .resource_mut::<NovaOsCloseTransition>()
        .closing = true;
    app.world_mut()
        .run_system_once(drive_nova_os_power_led)
        .unwrap();
    assert_eq!(
        bulb_color::<NovaOsPowerLedMarker>(&mut app),
        NOVA_OS_ORANGE,
        "the PWR LED turns orange while the monitor powers down"
    );
}
