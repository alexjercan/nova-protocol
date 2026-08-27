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

use super::support::{all_texts, entity_by_name, mods_app, set_state};
use crate::settings::{VolumeLabel, VolumeSlider};

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

/// The Settings panel builds real controls: the audio volume, the graphics
/// preset, and the read-only keybind reference. Structural (the panel is hidden
/// until toggled, but its entities exist), so it pins that the controls are
/// wired rather than placeheld.
/// Assertions are disk-independent: the loaded preset can be any saved value,
/// but exactly one button per group is always highlighted.
#[test]
fn settings_panel_builds_its_controls() {
    let mut app = mods_app();

    // The section headers and at least one keybind reference row render
    // (panel_header uppercases). "AUDIO"/"GRAPHICS"/"CONTROLS" + a control.
    let texts = all_texts(&mut app);
    for header in ["AUDIO", "GRAPHICS", "CONTROLS"] {
        assert!(
            texts.iter().any(|t| t == header),
            "the settings body is missing the {header} section"
        );
    }
    assert!(
        texts.iter().any(|t| t == "Main Drive"),
        "the keybind readout rows are missing (no Main Drive row)"
    );
    assert!(
        texts.iter().any(|t| t == "Pause / Menu"),
        "the declared-fixed rows are missing (no Pause / Menu row)"
    );
    // The mode toggles are registry rows, not fixed text: they reach this list
    // from the plugins that own them, so a rebind moves the readout.
    for action in ["NOVA OS", "HUD (On / Cinematic)"] {
        assert!(
            texts.iter().any(|t| t == action),
            "the {action} row is missing from the live bindings readout"
        );
    }

    // Exactly one volume slider, seeded to the current level, with a thumb
    // and a percent label.
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
    // The slider wears the shared `slider_track` block-meter (SliderBlock
    // bars), not a bespoke thumb.
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

    // One button per graphics tier, exactly one highlighted.
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

    unsafe { std::env::remove_var(nova_assets::storage::CONFIG_ROOT_ENV) };
    let _ = std::fs::remove_dir_all(&store);
}

/// The controls list is the LIVE table, not a copy of it. Rebind an action and
/// the row reads the new key on the next menu entry - which the hand-authored
/// KEYBINDS list could not do, and is why it went stale.
#[test]
fn the_controls_readout_follows_a_rebind() {
    let mut app = mods_app();
    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t.starts_with("W / Space")),
        "the shipped Main Drive row leads with its keyboard binding"
    );

    app.world_mut().resource_mut::<InputBindings>().register(
        ActionBinding::new("main_drive", "FLIGHT", "Main Drive")
            .keyboard([InputSource::Keyboard(KeyCode::KeyJ)])
            .gamepad([InputSource::Gamepad(GamepadButton::RightTrigger)]),
    );

    // The panel is built on menu entry, so leave and come back.
    set_state(&mut app, GameStates::Playing);
    set_state(&mut app, GameStates::MainMenu);

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "J   \u{b7}   Right Trigger"),
        "the Main Drive row still shows the old key; the readout is a mirror"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("W / Space")),
        "the old binding is still on screen"
    );
}

/// The whole shipped table, which exists in one place only here: `nova_menu`
/// is the crate that renders every group, so it is the only one that can see
/// all four owners' actions at once.
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
