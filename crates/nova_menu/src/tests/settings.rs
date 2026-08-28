//! The settings screen: that the controls build, that the skin buttons and the
//! volume slider write through to their resources and reskin live, and that an
//! edit made just before quitting is still persisted.

use bevy::{
    prelude::*,
    ui_widgets::{SliderValue, ValueChange},
};
use nova_gameplay::prelude::*;
use nova_input::prelude::{ActionBinding, InputBindings, InputSource};
use nova_ui::{
    prelude::UiSkin,
    widget::{
        button_on_setting, segmented_option, ButtonValue, Selected, SliderBlock, SliderFill,
        SLIDER_SEGMENTS,
    },
};

use super::support::{all_texts, entity_by_name, mods_app, shared_config_root};
use crate::settings::{
    SettingsActiveTab, SettingsControlsGroup, SettingsPanel, SettingsTab, SettingsTabKind,
    VolumeLabel, VolumeSlider, WindowModeSetting,
};

/// Open a settings tab and let the body reconcile. The tab BUTTON is exercised
/// by `pressing_a_tab_swaps_the_body`; every other test only wants the page.
fn open_tab(app: &mut App, tab: SettingsTabKind) {
    show_settings_panel(app);
    app.world_mut().resource_mut::<SettingsActiveTab>().0 = tab;
    app.update();
}

/// Put the Settings overlay on screen, the way the Settings button does.
///
/// The capture refuses to run without a visible panel, so a fixture that only
/// set the tab resource would arm a chip nothing could reach - which is the
/// production bug that gate exists to stop.
fn show_settings_panel(app: &mut App) {
    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsPanel>>()
        .iter(app.world())
        .next()
        .expect("the menu spawns a Settings panel");
    *app.world_mut()
        .get_mut::<Visibility>(panel)
        .expect("a panel is a UI node") = Visibility::Visible;
}

/// Close it again, the way Back does: `Visibility` only, no handler.
fn hide_settings_panel(app: &mut App) {
    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsPanel>>()
        .iter(app.world())
        .next()
        .expect("the menu spawns a Settings panel");
    *app.world_mut()
        .get_mut::<Visibility>(panel)
        .expect("a panel is a UI node") = Visibility::Hidden;
}

/// Open one Controls group. The tab shows a single binding group at a time, so
/// a test that wants a row outside FLIGHT has to say which page it is on. The
/// group BUTTON is exercised by `pressing_a_controls_group_swaps_the_rows`.
fn open_group(app: &mut App, group: &'static str) {
    app.world_mut().resource_mut::<SettingsControlsGroup>().0 = group;
    app.update();
}

/// DoD 2: pressing a `UI skin` segmented button (a `ThemedButton` carrying
/// `ButtonValue<UiSkin>`) drives the shared `UiSkin` resource + moves `Selected`,
/// through the same `button_on_setting` path as GraphicsQuality. Exercises the real
/// `segmented_button` factory.
#[test]
fn ui_skin_button_sets_resource() {
    let mut app = App::new();
    app.insert_resource(UiSkin::Phosphor);
    app.add_observer(button_on_setting::<UiSkin>);

    let phosphor = app
        .world_mut()
        .spawn((
            segmented_option("Phosphor"),
            ButtonValue(UiSkin::Phosphor),
            Selected,
        ))
        .id();
    let hardware = app
        .world_mut()
        .spawn((segmented_option("Hardware"), ButtonValue(UiSkin::Hardware)))
        .id();

    // Activate Hardware -> resource flips, selection moves off Phosphor.
    // flush: the observer moves `Selected` through `Commands`.
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: hardware });
    app.world_mut().flush();
    assert_eq!(*app.world().resource::<UiSkin>(), UiSkin::Hardware);
    assert!(app.world().entity(hardware).contains::<Selected>());
    assert!(
        !app.world().entity(phosphor).contains::<Selected>(),
        "the previous skin selection is cleared"
    );
}

/// The Settings panel builds real controls, one TAB at a time: the tab bar,
/// then each page's own widgets. Structural (the panel is hidden until
/// toggled, but its entities exist), so it pins that the controls are wired
/// rather than placeheld.
///
/// Assertions are disk-independent: the loaded preset can be any saved value,
/// but exactly one button per group is always highlighted.
#[test]
fn settings_panel_builds_one_tab_at_a_time() {
    let mut app = mods_app();

    // The bar carries every tab, with the open one highlighted.
    let tabs: Vec<(SettingsTabKind, bool)> = {
        let mut q = app.world_mut().query::<(&SettingsTab, Has<Selected>)>();
        q.iter(app.world()).map(|(tab, sel)| (tab.0, sel)).collect()
    };
    assert_eq!(tabs.len(), SettingsTabKind::ALL.len(), "one button per tab");
    assert_eq!(
        tabs.iter().filter(|(_, sel)| *sel).count(),
        1,
        "exactly one tab is open"
    );

    // AUDIO, the tab a fresh panel opens on: one volume slider, seeded in
    // range, wearing the shared block-meter, with a percent label.
    let slider_value = {
        let mut q = app
            .world_mut()
            .query_filtered::<&SliderValue, With<VolumeSlider>>();
        let values: Vec<f32> = q.iter(app.world()).map(|v| v.0).collect();
        assert_eq!(values.len(), 1, "exactly one volume slider");
        values[0]
    };
    assert!(
        (0.0..=1.0).contains(&slider_value),
        "the volume slider is seeded in range (got {slider_value})"
    );
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&Children, With<VolumeSlider>>();
        let bars = q.single(app.world()).map(|c| c.len()).unwrap_or(0);
        assert!(bars > 0, "the volume slider renders a block-meter");
    }
    {
        let mut q = app.world_mut().query_filtered::<(), With<VolumeLabel>>();
        assert_eq!(q.iter(app.world()).count(), 1, "one volume percent label");
    }

    // GRAPHICS: one button per tier, exactly one highlighted.
    open_tab(&mut app, SettingsTabKind::Graphics);
    let quality: Vec<bool> = {
        let mut q = app
            .world_mut()
            .query::<(&ButtonValue<GraphicsQuality>, Has<Selected>)>();
        q.iter(app.world()).map(|(_, sel)| sel).collect()
    };
    assert_eq!(
        quality.len(),
        GraphicsQuality::ALL.len(),
        "one button per graphics tier"
    );
    assert_eq!(
        quality.iter().filter(|&&s| s).count(),
        1,
        "exactly one graphics tier is highlighted as current"
    );
    assert!(
        all_texts(&mut app).iter().any(|t| t == "WINDOW"),
        "the window-mode group is on the graphics tab"
    );

    // CONTROLS: the live registry rows, one GROUP at a time, plus the fixed
    // chords in that group - and no slider, the audio page went away with its
    // tab.
    open_tab(&mut app, SettingsTabKind::Controls);
    assert!(
        all_texts(&mut app).iter().any(|t| t == "Main Drive"),
        "the tab opens on the first group"
    );
    open_group(&mut app, "SYSTEM");
    let texts = all_texts(&mut app);
    for row in ["Pause / Menu", "NOVA OS", "HUD (On / Cinematic)"] {
        assert!(
            texts.iter().any(|t| t == row),
            "the {row} row is missing from the SYSTEM group"
        );
    }
    assert!(
        !texts.iter().any(|t| t == "Main Drive"),
        "and a group shows only its own rows"
    );
    open_group(&mut app, "TARGETING");
    assert!(
        !all_texts(&mut app).iter().any(|t| t == "Radar (tap clear)"),
        "a shadow row moves with what it follows; it gets no row of its own"
    );
    {
        let mut q = app.world_mut().query_filtered::<(), With<VolumeSlider>>();
        assert_eq!(
            q.iter(app.world()).count(),
            0,
            "only the open tab is in the tree"
        );
    }

    // INTERFACE: one button per skin.
    open_tab(&mut app, SettingsTabKind::Interface);
    let skins: Vec<bool> = {
        let mut q = app
            .world_mut()
            .query::<(&ButtonValue<UiSkin>, Has<Selected>)>();
        q.iter(app.world()).map(|(_, sel)| sel).collect()
    };
    assert_eq!(skins.len(), 2, "one button per UI skin");
    assert_eq!(
        skins.iter().filter(|&&s| s).count(),
        1,
        "exactly one skin is highlighted as current"
    );
}

/// The tab BUTTON, not just the resource: a press swaps the body and moves the
/// highlight, which is the whole affordance.
#[test]
fn pressing_a_tab_swaps_the_body() {
    let mut app = mods_app();
    let controls = entity_by_name(&mut app, "Settings Tab: Controls").expect("the tab exists");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: controls });
    app.update();

    assert!(app.world().entity(controls).contains::<Selected>());
    assert!(
        all_texts(&mut app).iter().any(|t| t == "Main Drive"),
        "the controls page is up"
    );
}

/// Dragging the volume slider drives `MasterVolume` (which in turn drives
/// GlobalVolume + the thruster loop + persistence). The drag emits a
/// `ValueChange<f32>`; `on_volume_slider_change` must mirror it to the
/// resource. Delete that observer and this goes red.
#[test]
fn dragging_the_volume_slider_sets_master_volume() {
    let mut app = mods_app();
    let slider = entity_by_name(&mut app, "Volume Slider Track").expect("volume slider exists");
    app.world_mut().trigger(ValueChange::<f32> {
        source: slider,
        value: 0.3,
        is_final: true,
    });
    app.update();
    assert!(
        (app.world().resource::<MasterVolume>().0 - 0.3).abs() < 1e-6,
        "the slider value is mirrored onto MasterVolume (got {})",
        app.world().resource::<MasterVolume>().0
    );
}

/// DoD at the CALLER, not just the widget: the SHIPPED settings volume slider - the one
/// the owner played with - re-skins live and shows its value in the new skin. A widget-
/// level test proves the factory; this proves the wiring (lesson
/// `pin-each-caller-not-just-shared-core`).
#[test]
fn the_settings_volume_slider_reskins_live() {
    let mut app = mods_app();
    let slider = entity_by_name(&mut app, "Volume Slider Track").expect("volume slider exists");

    let child_kinds = |app: &mut App| -> (usize, usize) {
        let kids: Vec<Entity> = app
            .world()
            .entity(slider)
            .get::<Children>()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        let mut blocks = app.world_mut().query_filtered::<(), With<SliderBlock>>();
        let mut fills = app.world_mut().query_filtered::<(), With<SliderFill>>();
        let b = kids
            .iter()
            .filter(|&&c| blocks.get(app.world(), c).is_ok())
            .count();
        let f = kids
            .iter()
            .filter(|&&c| fills.get(app.world(), c).is_ok())
            .count();
        (b, f)
    };

    assert_eq!(
        child_kinds(&mut app),
        (SLIDER_SEGMENTS, 0),
        "phosphor: the segmented block-meter"
    );

    *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
    app.update();
    assert_eq!(
        child_kinds(&mut app),
        (0, 1),
        "hardware: one solid fill, live - not on the next settings-open"
    );

    // And the rebuilt fill carries the CURRENT volume, not a default.
    let value = {
        let mut q = app
            .world_mut()
            .query_filtered::<&SliderValue, With<VolumeSlider>>();
        q.single(app.world()).expect("one volume slider").0
    };
    let width = {
        let kids: Vec<Entity> = app
            .world()
            .entity(slider)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect();
        let mut q = app.world_mut().query_filtered::<&Node, With<SliderFill>>();
        kids.into_iter()
            .find_map(|c| q.get(app.world(), c).ok().map(|n| n.width))
            .expect("the fill exists")
    };
    assert_eq!(
        width,
        percent(value * 100.0),
        "the reskinned fill shows the current volume"
    );
}

/// The old standalone "Explore online (coming soon)" button was replaced by
/// the tab: neither its text nor its named entity may survive.
#[test]
fn the_old_coming_soon_button_is_gone() {
    let mut app = mods_app();
    let texts = all_texts(&mut app);
    assert!(
        !texts.iter().any(|t| t == "Explore online (coming soon)"),
        "the old coming-soon button text must not render anywhere"
    );
    assert!(
        entity_by_name(&mut app, "Explore Online Button").is_none(),
        "the old standalone button entity is gone"
    );
}

/// F22: a value edited inside the debounce window survives quitting.
///
/// `SETTINGS_SAVE_DEBOUNCE_FRAMES` is ~0.25s of idle frames and the Exit
/// button writes `AppExit` the same frame it is clicked, so without the `Last`
/// flush the pending write is simply never made. Delete
/// `flush_settings_on_exit` and the store stays at the pre-edit value.
#[test]
fn a_setting_edited_just_before_quitting_is_still_saved() {
    let store = std::env::temp_dir().join(format!("nova_menu_exit_flush_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    // SAFETY-BY-CONVENTION: the only test in this binary that writes the
    // settings store, and it must not touch the developer's real one.
    let shared_store = shared_config_root();
    unsafe { std::env::set_var(nova_assets::storage::CONFIG_ROOT_ENV, &store) };

    let mut app = mods_app();
    app.world_mut().insert_resource(MasterVolume(0.42));
    app.update();
    assert!(
        !store.join("settings.ron").exists(),
        "the debounce has not elapsed, so nothing is written yet"
    );

    app.world_mut().write_message(AppExit::Success);
    app.update();

    let saved = nova_assets::persist::load_from::<crate::settings_store::PersistedSettings>(
        &nova_assets::storage::NativeStorage::at(&store),
        crate::settings_store::KEY,
    )
    .expect("the exit flush wrote the pending settings");
    assert!(
        (saved.master_volume - 0.42).abs() < 1e-6,
        "the edited value is the one persisted (got {})",
        saved.master_volume
    );

    // RESTORE, do not remove: `isolate_the_config_store` sets this behind a
    // `Once`, so a removal here is permanent and every later fixture in this
    // binary would read the developer's real settings.ron.
    unsafe { std::env::set_var(nova_assets::storage::CONFIG_ROOT_ENV, &shared_store) };
    let _ = std::fs::remove_dir_all(&store);
}

/// The controls list is the LIVE table, not a copy of it. Rebind an action and
/// the row reads the new key on the next menu entry - which the hand-authored
/// KEYBINDS list could not do, and is why it went stale.
#[test]
fn the_controls_readout_follows_a_rebind() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    let shipped = all_texts(&mut app);
    for key in ["W", "Space"] {
        assert!(
            shipped.iter().any(|t| t == key),
            "the shipped Main Drive row shows its {key} binding"
        );
    }

    app.world_mut().resource_mut::<InputBindings>().register(
        ActionBinding::new("main_drive", "FLIGHT", "Main Drive")
            .keyboard([InputSource::Keyboard(KeyCode::KeyJ)])
            .gamepad([InputSource::Gamepad(GamepadButton::RightTrigger)]),
    );
    // No leaving and re-entering the menu: the body is reconciled, so a table
    // that moved reaches the open page on the next frame.
    app.update();

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "J"),
        "the Main Drive row still shows the old key; the readout is a mirror"
    );
    assert!(
        !texts.iter().any(|t| t == "Space"),
        "the old binding is still on screen"
    );
}

/// The group BUTTON, not just the resource: a press swaps the rows and moves
/// the highlight. Without it a player can only ever see the first group.
#[test]
fn pressing_a_controls_group_swaps_the_rows() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    let system = entity_by_name(&mut app, "Controls Group: SYSTEM").expect("the group exists");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: system });
    app.update();

    assert_eq!(app.world().resource::<SettingsControlsGroup>().0, "SYSTEM");
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "NOVA OS"),
        "the SYSTEM rows are up"
    );
    assert!(
        !texts.iter().any(|t| t == "Main Drive"),
        "and the group it came from is gone"
    );
    // The bar itself stays whole: a group tab must never hide the way back.
    assert!(
        entity_by_name(&mut app, "Controls Group: FLIGHT").is_some(),
        "every group is still one press away"
    );
}

/// A bound key is drawn as its KEYCAP, not spelled out - and an unmapped one
/// falls back to text rather than to an empty box.
///
/// `mods_app` runs no asset loading, so the glyph lookup is seeded here the
/// way `nova_assets` seeds it in the game: from the mapping table.
#[test]
fn a_bound_key_draws_its_keycap_and_an_unmapped_one_falls_back() {
    use nova_hud::prelude::{KeyGlyphs, NovaHudAssets};

    let mut app = mods_app();
    app.world_mut().insert_resource(NovaHudAssets {
        key_glyphs: KeyGlyphs::from_stems(|_| Some(Handle::default())),
        ..default()
    });
    open_tab(&mut app, SettingsTabKind::Controls);

    let chip = entity_by_name(&mut app, "Rebind: main_drive Desk").expect("the chip exists");
    let caps = |app: &mut App, chip: Entity| -> usize {
        let children: Vec<Entity> = app
            .world()
            .entity(chip)
            .get::<Children>()
            .map(|kids| kids.iter().collect())
            .unwrap_or_default();
        children
            .into_iter()
            .filter(|kid| app.world().entity(*kid).contains::<ImageNode>())
            .count()
    };
    assert_eq!(
        caps(&mut app, chip),
        2,
        "W and Space are drawn, not spelled"
    );
    // The pad column too, off its own pack - and its `A` must be the
    // controller's, not the keyboard key of the same name.
    let pad = entity_by_name(&mut app, "Rebind: autopilot_orbit Pad").expect("the chip exists");
    assert_eq!(caps(&mut app, pad), 1, "the pad face button is drawn");
    // The axis notes are keycaps too: RCS Aim is raw mouse motion, and the
    // read-only cell that says so draws the mouse rather than spelling it.
    let aim = entity_by_name(&mut app, "Controls Cell: rcs_aim Desk").expect("the cell exists");
    let slot = app
        .world()
        .entity(aim)
        .get::<Children>()
        .and_then(|kids| kids.iter().next())
        .expect("the cell holds its chip row");
    assert_eq!(caps(&mut app, slot), 1, "the mouse note is drawn too");
    assert!(
        !all_texts(&mut app).iter().any(|t| t == "W"),
        "and the text chip is gone where a picture took its place"
    );

    // F13 has no art in the pack. The row must still say what it is bound to.
    app.world_mut().resource_mut::<InputBindings>().register(
        ActionBinding::new("main_drive", "FLIGHT", "Main Drive")
            .keyboard([InputSource::Keyboard(KeyCode::F13)]),
    );
    app.update();
    let chip = entity_by_name(&mut app, "Rebind: main_drive Desk").expect("the chip exists");
    assert_eq!(caps(&mut app, chip), 0, "no keycap for an unmapped key");
    assert!(
        all_texts(&mut app).iter().any(|t| t == "F13"),
        "the unmapped key falls back to its name"
    );
}

/// Arm a chip the way a player does - by clicking it - and let the arming
/// click clear, which is what the capture waits for.
fn arm_chip(app: &mut App, name: &str) {
    let chip = entity_by_name(app, name).unwrap_or_else(|| panic!("no chip named {name}"));
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: chip });
    app.update();
}

/// Press one key for a frame, then let go.
fn tap_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    keys.release(key);
}

/// The keyboard half of the DoD: arm a flight row, press a free key, and both
/// the table and the row that draws it move.
#[test]
fn a_rebind_takes_the_next_key_and_the_table_follows() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Desk");
    assert!(
        all_texts(&mut app).iter().any(|t| t == "PRESS A KEY"),
        "the armed chip prompts for the press"
    );

    tap_key(&mut app, KeyCode::KeyJ);

    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![InputSource::Keyboard(KeyCode::KeyJ)],
        "the whole desk column is what the chip shows, so the whole column moves"
    );
    assert!(
        all_texts(&mut app).iter().any(|t| t == "J"),
        "the row redrew on the new key"
    );
}

/// A pad button the shipped table leaves free, ASKED FOR rather than pinned: the
/// table keeps growing, and a hard-coded button turns every future binding into
/// a confusing failure in a test that is not about conflicts.
fn free_pad_button(app: &App, action: &str) -> GamepadButton {
    let bindings = app.world().resource::<InputBindings>();
    [
        GamepadButton::LeftTrigger2,
        GamepadButton::RightTrigger2,
        GamepadButton::C,
        GamepadButton::Z,
        GamepadButton::Mode,
    ]
    .into_iter()
    .find(|button| {
        bindings
            .conflict_for(action, InputSource::Gamepad(*button))
            .is_none()
    })
    .expect("the shipped table leaves at least one pad button free")
}

/// The genuinely new half: a pad button, which lives on the `Gamepad`
/// COMPONENT - bevy 0.19 registers no `ButtonInput<GamepadButton>` for a
/// rebind screen to read.
#[test]
fn a_rebind_takes_a_pad_button() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    let button = free_pad_button(&app, "autopilot_stop");
    arm_chip(&mut app, "Rebind: autopilot_stop Pad");

    app.world_mut()
        .entity_mut(pad)
        .get_mut::<Gamepad>()
        .expect("just spawned")
        .digital_mut()
        .press(button);
    app.update();

    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("autopilot_stop")
            .expect("registered")
            .gamepad,
        vec![InputSource::Gamepad(button)],
    );
}

/// A key another action already holds in the same live set is REFUSED, with
/// the name of what holds it, and the chip stays armed so the next press is
/// still the rebind.
#[test]
fn a_taken_key_is_refused_by_name_and_the_chip_stays_armed() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Desk");

    // X is Autopilot: Stop, and flight is one live set.
    tap_key(&mut app, KeyCode::KeyX);
    app.update();

    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![
            InputSource::Keyboard(KeyCode::KeyW),
            InputSource::Keyboard(KeyCode::Space)
        ],
        "the refused capture left the binding alone"
    );
    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t == "X is already bound to Autopilot: Stop"),
        "the refusal names what holds the key"
    );
    assert!(
        all_texts(&mut app).iter().any(|t| t == "PRESS A KEY"),
        "and the chip is still waiting"
    );

    // The next press, on a free key, still lands.
    tap_key(&mut app, KeyCode::KeyJ);
    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![InputSource::Keyboard(KeyCode::KeyJ)]
    );
}

/// The other direction of the same guard. A section's trigger is authored on
/// the ship, not registered as an action, so the table cannot see it - and
/// every base scenario arms its turrets on the right trigger. Binding Main
/// Drive there would make one pull burn AND fire.
#[test]
fn a_key_a_live_ship_section_holds_is_refused_too() {
    use nova_events::prelude::EntityId;
    use nova_ship::prelude::SpaceshipTurretInputBinding;

    let mut app = mods_app();
    app.world_mut().spawn((
        EntityId::new("turret_dorsal"),
        SpaceshipTurretInputBinding(vec![InputSource::Gamepad(GamepadButton::RightTrigger2)]),
    ));
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Pad");

    let pad = app.world_mut().spawn(Gamepad::default()).id();
    app.world_mut()
        .entity_mut(pad)
        .get_mut::<Gamepad>()
        .expect("just spawned")
        .digital_mut()
        .press(GamepadButton::RightTrigger2);
    app.update();

    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .gamepad,
        vec![InputSource::Gamepad(GamepadButton::RightTrigger)],
        "the capture is refused, not written"
    );
    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t == "Right Trigger 2 is already bound to the ship's turret_dorsal section"),
        "and the row says which section holds it"
    );
}

/// Escape backs out of a capture. It is the reason no row can bind it.
#[test]
fn escape_backs_out_of_an_armed_rebind() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Desk");

    tap_key(&mut app, KeyCode::Escape);
    app.update();

    assert!(
        !all_texts(&mut app).iter().any(|t| t == "PRESS A KEY"),
        "the capture is disarmed"
    );
    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![
            InputSource::Keyboard(KeyCode::KeyW),
            InputSource::Keyboard(KeyCode::Space)
        ],
        "and nothing moved"
    );
}

/// Closing the panel with a chip armed must drop the capture.
///
/// The capture is ungated by menu state - the pause overlay shows the same body
/// - and Back only flips `Visibility`, so nothing else lowers the arm. Left
/// unfixed, a player who armed a chip and changed their mind had the next key
/// they pressed in flight written into the table and persisted to disk, with no
/// prompt on screen and Reset Defaults the only way back.
#[test]
fn closing_the_panel_drops_an_armed_rebind() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Desk");

    hide_settings_panel(&mut app);
    app.update();

    // `A` is free in FLIGHT: only `novaos_pan_left` holds it, and Viewer does
    // not overlap Flight, so the capture would have ACCEPTED it.
    tap_key(&mut app, KeyCode::KeyA);
    app.update();

    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![
            InputSource::Keyboard(KeyCode::KeyW),
            InputSource::Keyboard(KeyCode::Space)
        ],
        "the key pressed after the panel closed is not captured"
    );
}

/// The way back. A row rebound onto a key a player can no longer find is
/// otherwise permanent, because the only surface that moves it is bound to it.
#[test]
fn reset_defaults_puts_every_row_back() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Desk");
    tap_key(&mut app, KeyCode::KeyJ);
    assert!(!app
        .world()
        .resource::<InputBindings>()
        .overrides()
        .is_empty());

    let reset = entity_by_name(&mut app, "Reset Bindings").expect("the reset button exists");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: reset });
    app.update();

    assert!(
        app.world()
            .resource::<InputBindings>()
            .overrides()
            .is_empty(),
        "every row is back on what it shipped with"
    );
}

/// A rebind of the hold half takes the tap half with it: one gesture, one row,
/// and a rig that would otherwise read the two halves off different keys.
#[test]
fn rebinding_a_gesture_moves_the_half_that_follows_it() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    open_group(&mut app, "TARGETING");
    arm_chip(&mut app, "Rebind: radar_hold Desk");
    tap_key(&mut app, KeyCode::KeyJ);

    let table = app.world().resource::<InputBindings>();
    assert_eq!(
        table.get("radar_clear").expect("registered").keyboard,
        vec![InputSource::Keyboard(KeyCode::KeyJ)],
    );
}

/// The Graphics row that has nowhere else to live: the window is created once
/// at a fixed size, so the only way out of a 1024x768 frame is this button. It
/// must move the LIVE window, not just the saved value.
#[test]
fn the_window_row_drives_the_primary_window() {
    use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

    let mut app = mods_app();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    open_tab(&mut app, SettingsTabKind::Graphics);

    let borderless =
        entity_by_name(&mut app, "Window Borderless").expect("the Graphics tab offers the mode");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: borderless });
    app.update();

    assert_eq!(
        *app.world().resource::<WindowModeSetting>(),
        WindowModeSetting::Borderless
    );
    assert_eq!(
        app.world().entity(window).get::<Window>().unwrap().mode,
        WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        "the live window followed the row"
    );

    let windowed = entity_by_name(&mut app, "Window Windowed").expect("and the way back out of it");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: windowed });
    app.update();
    assert_eq!(
        app.world().entity(window).get::<Window>().unwrap().mode,
        WindowMode::Windowed
    );
}

/// The pointer's own button is refused. Found live: an armed chip ate the next
/// click a walk made on another control, and `main_drive` came out bound to
/// Left Mouse - which then made the row that would undo it unclickable.
#[test]
fn an_armed_chip_does_not_eat_the_pointers_own_button() {
    let mut app = mods_app();
    open_tab(&mut app, SettingsTabKind::Controls);
    arm_chip(&mut app, "Rebind: main_drive Desk");

    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();

    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![
            InputSource::Keyboard(KeyCode::KeyW),
            InputSource::Keyboard(KeyCode::Space)
        ],
        "the click did not become the binding"
    );
    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t == "Left Mouse stays the pointer"),
        "and the row says why"
    );

    // Another mouse button is still fair game (Right Mouse is taken - it
    // raises the weapons - so the free one is the wheel click).
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.release(MouseButton::Left);
        mouse.press(MouseButton::Middle);
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<InputBindings>()
            .get("main_drive")
            .expect("registered")
            .keyboard,
        vec![InputSource::Mouse(MouseButton::Middle)],
    );
}

/// The other entry point. The pause overlay builds its OWN settings body, so a
/// tab bar wired only into the main menu would leave the in-flight screen with
/// no way off the Audio tab - and it is the only rebind surface a player can
/// reach while a ship is flying.
#[test]
fn the_pause_overlay_settings_body_tabs_too() {
    use super::support::{dummy_scenarios, enter_playing, press_escape};

    let mut app = mods_app();
    app.insert_resource(dummy_scenarios());
    enter_playing(&mut app);
    press_escape(&mut app);

    let controls =
        entity_by_name(&mut app, "Settings Tab: Controls").expect("the pause body has the tabs");
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: controls });
    app.update();

    assert_eq!(
        app.world().resource::<SettingsActiveTab>().0,
        SettingsTabKind::Controls
    );
    assert!(
        entity_by_name(&mut app, "Rebind: main_drive Desk").is_some(),
        "and the rows the flying player came here for"
    );
}

/// The whole shipped table, which exists in one place only here: `nova_menu`
/// is the crate that renders every group, so it is the only one that can see
/// all five owners' actions at once. `scenario_advance` is the one the guard
/// used to miss: it declares `Flight`, so it shares a live set with every
/// flight action, and its owner sat one crate away from the check.
///
/// A key held by two actions is normal and deliberate - `G` is go-to in
/// flight, GOTO in the map viewer and the mates overlay in the ship viewer,
/// and only one of the three is ever listening. What must not happen is two
/// actions holding one source INSIDE a live set, because nothing consumes an
/// input: the key would drive both.
///
/// `radar_clear` FOLLOWS `radar_hold` - one gesture read two ways, hold to
/// search and tap to clear - so the table itself knows that pair shares its
/// key on purpose and this test carries no exemption list.
#[test]
fn no_two_actions_that_can_be_live_together_share_a_source() {
    let mut table = InputBindings::from_actions(nova_ship::input::bindings::flight_bindings());
    for action in nova_ship::input::bindings::camera_bindings()
        .into_iter()
        .chain(nova_hud::hud_bindings())
        .chain(nova_os_ui::bindings::novaos_bindings())
        .chain(nova_scenario::prelude::scenario_bindings())
    {
        table.register(action);
    }

    let found: Vec<String> = table
        .conflicts()
        .into_iter()
        .map(|(one, other, source)| {
            format!(
                "`{}` ({:?}) and `{}` ({:?}) both hold {}",
                one.name,
                one.context,
                other.name,
                other.context,
                source.label()
            )
        })
        .collect();
    assert!(found.is_empty(), "{}", found.join("; "));
}

/// The reason the shared keys are legal, stated as a test: exactly one of the
/// three `G` actions is live at any instant, and which one depends on what
/// owns the screen.
#[test]
fn only_one_of_the_three_actions_bound_to_g_is_ever_live() {
    use nova_input::prelude::{ActionContext, ActiveContexts};

    let mut table = InputBindings::from_actions(nova_ship::input::bindings::flight_bindings());
    for action in nova_os_ui::bindings::novaos_bindings() {
        table.register(action);
    }
    let on_g = |active: &ActiveContexts| -> Vec<&'static str> {
        table
            .live(active)
            .filter(|action| {
                action
                    .keyboard
                    .contains(&InputSource::Keyboard(KeyCode::KeyG))
            })
            .map(|action| action.name)
            .collect()
    };

    let mut active = ActiveContexts::default();
    assert!(
        on_g(&active).is_empty(),
        "at the prompt `G` is a character being typed, not an action"
    );

    active.set(ActionContext::Flight, true);
    assert_eq!(on_g(&active), vec!["autopilot_goto"]);

    active.set(ActionContext::Flight, false);
    active.set(ActionContext::Viewer, true);
    active.set(ActionContext::ViewerApp("map"), true);
    assert_eq!(on_g(&active), vec!["map_goto"]);

    active.set(ActionContext::ViewerApp("map"), false);
    active.set(ActionContext::ViewerApp("ship"), true);
    assert_eq!(on_g(&active), vec!["ship_mates"]);
}
