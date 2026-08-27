//! The Settings panel: the tab bar and body spawned by both the main menu and
//! the pause overlay, the rebind capture, and the persistence systems.
//!
//! The body is RECONCILED, not built once. Four tabs share one container and
//! [`refresh_settings_tab`] fills it from the live resources, so a rebind, a
//! tab press or a loaded store all reach the screen through the same path.

use bevy::{
    prelude::*,
    ui_widgets::{
        observe, Activate, Slider, SliderRange, SliderStep, SliderValue, TrackClick, ValueChange,
    },
};
use nova_gameplay::prelude::*;
use nova_input::prelude::*;
use nova_os_ui::prelude::NovaOsMonitorSettings;
use nova_ui::{
    prelude::UiSkin,
    theme,
    widget::{
        panel_header, segmented_container, segmented_option, separator, slider_track, ButtonValue,
        Selected, UiText,
    },
};
use serde::{Deserialize, Serialize};

use crate::settings_store::{load_settings, save_settings, PersistedSettings};

/// Marker for the main-menu Settings panel, toggled by the Settings button.
#[derive(Component)]
pub(crate) struct SettingsPanel;

/// Marker for the pause-menu Settings panel (the same modal reached from the
/// pause overlay, user note 2026-07-16), toggled by the pause Settings button.
#[derive(Component)]
pub(crate) struct PauseSettingsPanel;

pub(crate) fn on_settings(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<SettingsPanel>>,
) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

pub(crate) fn on_settings_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<SettingsPanel>>,
) {
    **panel = Visibility::Hidden;
}

/// The master-volume [`Slider`] entity (bevy's headless slider widget), so the
/// change observer and the thumb/label sync system can find it.
#[derive(Component)]
pub(crate) struct VolumeSlider;

/// The "72%" readout beside the volume slider.
#[derive(Component)]
pub(crate) struct VolumeLabel;

/// Format a linear volume factor as a whole-percent label.
pub(crate) fn volume_label(value: f32) -> String {
    format!("{}%", (value.clamp(0.0, 1.0) * 100.0).round() as i32)
}

/// One page of the settings panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SettingsTabKind {
    /// Master volume.
    #[default]
    Audio,
    /// The quality preset and the window mode.
    Graphics,
    /// Every rebindable action, plus the fixed system chords.
    Controls,
    /// The UI skin.
    Interface,
}

impl SettingsTabKind {
    /// The tabs, in bar order.
    pub(crate) const ALL: [Self; 4] =
        [Self::Audio, Self::Graphics, Self::Controls, Self::Interface];

    /// What the tab button reads.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Graphics => "Graphics",
            Self::Controls => "Controls",
            Self::Interface => "Interface",
        }
    }
}

/// The open tab. Deliberately NOT reset when the panel opens: a player who
/// came back to move one more keybind lands where they left off.
#[derive(Resource, Default, PartialEq, Eq)]
pub(crate) struct SettingsActiveTab(pub(crate) SettingsTabKind);

/// A tab-bar button: the tab it opens.
#[derive(Component)]
pub(crate) struct SettingsTab(pub(crate) SettingsTabKind);

/// The container [`refresh_settings_tab`] owns the children of. Both entry
/// points put it on their scrolling body, so one reconciler fills both.
#[derive(Component)]
pub(crate) struct SettingsTabBody;

/// Which half of a settings row a rebind is capturing for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RebindDevice {
    /// Keyboard and mouse buttons - one column, because they are one hand.
    Desk,
    /// Gamepad buttons.
    Pad,
}

impl RebindDevice {
    /// The prompt the armed chip shows.
    fn prompt(self) -> &'static str {
        match self {
            Self::Desk => "PRESS A KEY",
            Self::Pad => "PRESS A BUTTON",
        }
    }
}

/// A rebind chip: which action and which half of it a click arms.
#[derive(Component, Clone, Copy)]
pub(crate) struct RebindChip {
    /// The registry action name.
    pub(crate) action: &'static str,
    /// The column this chip owns.
    pub(crate) device: RebindDevice,
}

/// The Reset Defaults button at the foot of the Controls tab.
#[derive(Component)]
pub(crate) struct ResetBindings;

/// The armed rebind and the last refusal.
///
/// The capture is a resource rather than a component on the chip because it is
/// exclusive: one chip is armed at a time, and every device surface answers to
/// it whether or not the pointer is still over the row.
#[derive(Resource, Default)]
pub(crate) struct PendingRebind {
    armed: Option<ArmedRebind>,
    /// Why the last capture was refused, shown under the rows until the next
    /// one is armed.
    refusal: Option<String>,
}

/// The chip waiting for a press.
struct ArmedRebind {
    action: &'static str,
    device: RebindDevice,
    /// The click that armed this is still down. Capturing now would take the
    /// arming press itself, so the capture waits for a clean frame.
    awaiting_release: bool,
}

impl PendingRebind {
    /// Whether this exact chip is the armed one.
    fn armed_on(&self, action: &str, device: RebindDevice) -> bool {
        self.armed
            .as_ref()
            .is_some_and(|armed| armed.action == action && armed.device == device)
    }
}

/// Whether the window fills the screen. Native only: the web build already
/// fits its canvas, and a browser cannot go fullscreen without a user gesture
/// the settings row does not carry.
///
/// `Resource`-only on purpose: on Bevy 0.19 a `#[derive(Resource)]` type is
/// component-backed, so this doubles as the `Component` that
/// `button_on_setting::<WindowModeSetting>` needs, and deriving `Component`
/// too would conflict.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowModeSetting {
    /// A 1024x768 window, what the game has always launched as.
    #[default]
    Windowed,
    /// Borderless, filling the monitor the window is on.
    Borderless,
}

impl WindowModeSetting {
    /// The modes, in row order.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "the row is native-only; the value still persists on the web"
        )
    )]
    const ALL: [Self; 2] = [Self::Windowed, Self::Borderless];

    /// What the option reads.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "the row is native-only; the value still persists on the web"
        )
    )]
    fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::Borderless => "Borderless",
        }
    }
}

/// The tab bar: one segmented row above the scrolling body. Spawned by each
/// entry point beside its own [`SettingsTabBody`], so a tab press reaches both.
pub(crate) fn build_settings_tabs(
    parent: &mut ChildSpawnerCommands,
    skin: UiSkin,
    active: SettingsTabKind,
) {
    parent
        .spawn((
            Name::new("Settings Tab Bar"),
            Node {
                margin: UiRect::bottom(px(10)),
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn((Name::new("Settings Tabs"), segmented_container(skin)))
                .with_children(|row| {
                    for tab in SettingsTabKind::ALL {
                        let mut button = row.spawn((
                            Name::new(format!("Settings Tab: {}", tab.label())),
                            segmented_option(tab.label()),
                            SettingsTab(tab),
                            observe(on_settings_tab),
                        ));
                        if tab == active {
                            button.insert(Selected);
                        }
                    }
                });
        });
}

/// Open the clicked tab and move the `Selected` highlight onto it.
pub(crate) fn on_settings_tab(
    activate: On<Activate>,
    tabs: Query<(Entity, &SettingsTab)>,
    mut active: ResMut<SettingsActiveTab>,
    mut rebind: ResMut<PendingRebind>,
    mut commands: Commands,
) {
    let Ok((entity, tab)) = tabs.get(activate.entity) else {
        return;
    };
    if active.0 == tab.0 {
        return;
    }
    active.0 = tab.0;
    // Leaving Controls with a chip armed would leave the next key press
    // captured by a row nothing is showing.
    disarm(&mut rebind);
    for (other, _) in &tabs {
        commands.entity(other).remove::<Selected>();
    }
    commands.entity(entity).insert(Selected);
}

/// Whether [`refresh_settings_tab`] has anything to redraw.
///
/// Deliberately NOT armed by `MasterVolume`: the slider mutates it every frame
/// of a drag, and rebuilding the body under the pointer would drop the drag.
/// The other settings move their own `Selected` highlight through
/// `button_on_setting`, so they need no rebuild either.
pub(crate) fn settings_tab_dirty(
    active: Res<SettingsActiveTab>,
    bindings: Res<InputBindings>,
    rebind: Res<PendingRebind>,
    spawned: Query<(), Added<SettingsTabBody>>,
) -> bool {
    active.is_changed() || bindings.is_changed() || rebind.is_changed() || !spawned.is_empty()
}

/// Fill every [`SettingsTabBody`] with the open tab.
pub(crate) fn refresh_settings_tab(
    mut commands: Commands,
    bodies: Query<Entity, With<SettingsTabBody>>,
    active: Res<SettingsActiveTab>,
    volume: Res<MasterVolume>,
    quality: Res<GraphicsQuality>,
    skin: Res<UiSkin>,
    window_mode: Res<WindowModeSetting>,
    bindings: Res<InputBindings>,
    rebind: Res<PendingRebind>,
) {
    for body in &bodies {
        commands.entity(body).despawn_related::<Children>();
        commands.entity(body).with_children(|list| match active.0 {
            SettingsTabKind::Audio => build_audio_tab(list, *volume, *skin),
            SettingsTabKind::Graphics => build_graphics_tab(list, *quality, *window_mode, *skin),
            SettingsTabKind::Controls => build_controls_tab(list, &bindings, &rebind),
            SettingsTabKind::Interface => build_interface_tab(list, *skin),
        });
    }
}

/// AUDIO - master volume as a draggable slider (bevy's headless `Slider`; drag
/// handling comes from `UiWidgetsPlugins` in DefaultPlugins, the value is
/// committed by `slider_self_update` and mirrored to `MasterVolume` by
/// `on_volume_slider_change`, both registered in the plugin).
fn build_audio_tab(list: &mut ChildSpawnerCommands, volume: MasterVolume, skin: UiSkin) {
    list.spawn((
        Name::new("Volume Row"),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            margin: UiRect::vertical(px(4)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Name::new("Volume Slider"),
            UiText,
            Text::new("Volume"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
            Node {
                min_width: px(70),
                ..default()
            },
        ));
        // The slider: a `bevy_ui_widgets::Slider` wearing the shared
        // `slider_track` (shown by nova_ui's `sync_slider_tracks`, which lights
        // the phosphor block-meter and moves the hardware fill).
        // Wrapped in a flex-grow cell so the 100%-wide track fills the row's
        // middle. `Snap` so a click on the track jumps to that spot.
        row.spawn(Node {
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|cell| {
            cell.spawn((
                Name::new("Volume Slider Track"),
                VolumeSlider,
                Slider {
                    track_click: TrackClick::Snap,
                    ..default()
                },
                SliderValue(volume.factor()),
                SliderRange::new(0.0, 1.0),
                SliderStep(0.05),
                slider_track(volume.factor(), skin),
            ));
        });
        row.spawn((
            Name::new("Volume Label"),
            VolumeLabel,
            UiText,
            Text::new(volume_label(volume.factor())),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
            Node {
                min_width: px(44),
                ..default()
            },
        ));
    });
}

/// GRAPHICS - the quality preset (each tier drives the combat juice; the
/// low-end mode extends what Low/Medium skip) and, on native, the window mode.
fn build_graphics_tab(
    list: &mut ChildSpawnerCommands,
    quality: GraphicsQuality,
    window_mode: WindowModeSetting,
    skin: UiSkin,
) {
    list.spawn(panel_header("Quality"));
    list.spawn((Name::new("Graphics Row"), segmented_container(skin)))
        .with_children(|row| {
            for tier in GraphicsQuality::ALL {
                let mut button = row.spawn((
                    Name::new(format!("Graphics {}", tier.label())),
                    segmented_option(tier.label()),
                    ButtonValue(tier),
                ));
                if tier == quality {
                    button.insert(Selected);
                }
            }
        });

    // The web build fits its canvas already, and a browser will not go
    // fullscreen without a user gesture this row cannot supply - so the row is
    // absent there rather than present and inert.
    #[cfg(not(target_arch = "wasm32"))]
    {
        list.spawn(separator());
        list.spawn(panel_header("Window"));
        list.spawn((Name::new("Window Mode Row"), segmented_container(skin)))
            .with_children(|row| {
                for mode in WindowModeSetting::ALL {
                    let mut button = row.spawn((
                        Name::new(format!("Window {}", mode.label())),
                        segmented_option(mode.label()),
                        ButtonValue(mode),
                    ));
                    if mode == window_mode {
                        button.insert(Selected);
                    }
                }
            });
    }
    #[cfg(target_arch = "wasm32")]
    let _ = window_mode;
}

/// INTERFACE - the UI skin choice. A segmented Phosphor|Hardware control wired
/// through `ButtonValue<UiSkin>` + the app-global `button_on_setting::<UiSkin>`
/// observer, exactly like the graphics preset.
fn build_interface_tab(list: &mut ChildSpawnerCommands, skin: UiSkin) {
    list.spawn((Name::new("UI Skin Row"), segmented_container(skin)))
        .with_children(|row| {
            for option in [UiSkin::Phosphor, UiSkin::Hardware] {
                let label = match option {
                    UiSkin::Phosphor => "Phosphor",
                    UiSkin::Hardware => "Hardware",
                };
                let mut button = row.spawn((
                    Name::new(format!("UI Skin {label}")),
                    segmented_option(label),
                    ButtonValue(option),
                ));
                if option == skin {
                    button.insert(Selected);
                }
            }
        });
}

/// CONTROLS - every rebindable action off the LIVE table, then the chords that
/// are not actions at all.
///
/// A shadow row (`radar_clear`) is absent: it moves with the action it
/// follows, so showing it would offer a second way to break one gesture.
fn build_controls_tab(
    list: &mut ChildSpawnerCommands,
    bindings: &InputBindings,
    rebind: &PendingRebind,
) {
    let mut groups = bindings.groups();
    for (group, ..) in FIXED_ROWS {
        if !groups.contains(group) {
            groups.push(group);
        }
    }
    for group in groups {
        list.spawn((
            Name::new(format!("Controls Group: {group}")),
            UiText,
            Text::new(group),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
            Node {
                margin: UiRect::top(px(6)),
                ..default()
            },
        ));
        for action in bindings.rows().filter(|action| action.group == group) {
            spawn_rebind_row(list, action, rebind);
        }
        for (_, label, keyboard, gamepad) in FIXED_ROWS.iter().filter(|(fixed, ..)| *fixed == group)
        {
            spawn_keybind_row(list, label, keyboard, gamepad);
        }
    }

    list.spawn(separator());
    if let Some(reason) = &rebind.refusal {
        list.spawn((
            Name::new("Rebind Refusal"),
            UiText,
            Text::new(reason.clone()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::AMBER_NOVA),
            Node {
                margin: UiRect::vertical(px(4)),
                ..default()
            },
        ));
    }
    list.spawn((
        Name::new("Reset Bindings"),
        segmented_option("Reset Defaults"),
        ResetBindings,
        observe(on_reset_bindings),
    ));
}

/// Put every action back on what it shipped with. The only way out of a remap
/// a player cannot undo by hand - a row rebound onto a key they can no longer
/// find is otherwise permanent.
pub(crate) fn on_reset_bindings(
    _activate: On<Activate>,
    mut bindings: ResMut<InputBindings>,
    mut rebind: ResMut<PendingRebind>,
) {
    let names: Vec<&'static str> = bindings.names().collect();
    for name in names {
        bindings.reset(name);
    }
    disarm(&mut rebind);
}

/// Arm the clicked chip. The click itself is still down, so the capture waits
/// for a clean frame before reading a press.
pub(crate) fn on_rebind_chip(
    activate: On<Activate>,
    chips: Query<&RebindChip>,
    mut rebind: ResMut<PendingRebind>,
) {
    let Ok(chip) = chips.get(activate.entity) else {
        return;
    };
    rebind.armed = Some(ArmedRebind {
        action: chip.action,
        device: chip.device,
        awaiting_release: true,
    });
    rebind.refusal = None;
}

/// Take the next press for the armed chip.
///
/// Escape cancels - it is the back-out on every capture surface in the game,
/// which is why no row can bind it. A press something else already holds in
/// the same live set is REFUSED with the name of what holds it, and the chip
/// stays armed so the next press is still the rebind.
pub(crate) fn apply_settings_rebind(
    sources: InputSources,
    keys: Res<ButtonInput<KeyCode>>,
    mut rebind: ResMut<PendingRebind>,
    mut bindings: ResMut<InputBindings>,
) {
    let Some((action, device, awaiting_release)) = rebind
        .armed
        .as_ref()
        .map(|armed| (armed.action, armed.device, armed.awaiting_release))
    else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        disarm(&mut rebind);
        return;
    }
    if awaiting_release {
        if sources.all_released() {
            if let Some(armed) = rebind.armed.as_mut() {
                armed.awaiting_release = false;
            }
        }
        return;
    }

    let captured = match device {
        RebindDevice::Desk => sources.captured_desk(),
        RebindDevice::Pad => sources.captured_pad(),
    };
    let Some(source) = captured else {
        return;
    };
    // The pointer's own button is never taken. Every other control on this
    // screen is clicked with it, so an armed chip would otherwise eat the next
    // click a player made anywhere - and a game whose main drive is Left Mouse
    // cannot be un-bound, because the row that would fix it needs a click.
    if source == InputSource::Mouse(MouseButton::Left) {
        rebind.refusal = Some("Left Mouse stays the pointer".to_string());
        return;
    }

    if let Some(taken_by) = bindings.conflict_for(action, source) {
        let reason = format!("{} is already {}", source.label(), taken_by.label);
        rebind.refusal = Some(reason);
        return;
    }

    let Some(current) = bindings.get(action) else {
        disarm(&mut rebind);
        return;
    };
    // The whole column moves, not just its first entry: the chip shows one
    // column and a player who presses one key means that column is now that
    // key. `Reset Defaults` is what puts a multi-key default back.
    let mut spec = current.spec();
    match device {
        RebindDevice::Desk => spec.keyboard = vec![source],
        RebindDevice::Pad => spec.gamepad = vec![source],
    }
    bindings.rebind(action, spec);
    disarm(&mut rebind);
}

/// Drop the armed chip and the refusal beside it, marking the resource changed
/// exactly once so the body redraws.
fn disarm(rebind: &mut ResMut<'_, PendingRebind>) {
    if rebind.armed.is_some() || rebind.refusal.is_some() {
        rebind.armed = None;
        rebind.refusal = None;
    }
}

/// One rebindable row: the action, then a chip per device column.
///
/// A column with no source of its own is not a chip. `rcs_aim` is raw mouse
/// motion and `camera_rotate` is motion plus a stick: an action is MOVED here,
/// never given a button it never had, because a key bound to a `Vec2` action
/// would read as bound and do nothing.
fn spawn_rebind_row(
    list: &mut ChildSpawnerCommands,
    action: &ActionBinding,
    rebind: &PendingRebind,
) {
    list.spawn((
        Name::new(format!("Keybind: {}", action.label)),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: UiRect::axes(px(2), px(3)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            UiText,
            Text::new(action.label.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
            // The label takes ALL the leftover width, so the two chip columns
            // land at the same x on every row. Sized by the label instead, a
            // long verb pushed its chips right and the column read as ragged.
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                ..default()
            },
        ));
        spawn_chip(
            row,
            action,
            RebindDevice::Desk,
            !action.keyboard.is_empty(),
            &action.keyboard_display(),
            rebind,
        );
        spawn_chip(
            row,
            action,
            RebindDevice::Pad,
            !action.gamepad.is_empty(),
            &action.gamepad_display(),
            rebind,
        );
    });
}

/// One device column of a rebind row: a button when the action holds a source
/// there, plain text when it does not.
fn spawn_chip(
    row: &mut ChildSpawnerCommands,
    action: &ActionBinding,
    device: RebindDevice,
    rebindable: bool,
    display: &str,
    rebind: &PendingRebind,
) {
    let armed = rebind.armed_on(action.name, device);
    let text = if armed {
        device.prompt()
    } else if display.is_empty() {
        "-"
    } else {
        display
    };
    // A fixed-width CELL, not a fixed-width chip: `segmented_option` brings its
    // own `Node` and a second one would replace the button's padding with this.
    let mut cell = row.spawn((
        Name::new(format!("Controls Cell: {} {:?}", action.name, device)),
        Node {
            width: px(CHIP_WIDTH),
            ..default()
        },
    ));
    if !rebindable {
        cell.with_children(|cell| {
            cell.spawn((
                UiText,
                Text::new(text.to_string()),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ));
        });
        return;
    }
    cell.with_children(|cell| {
        let mut chip = cell.spawn((
            Name::new(format!("Rebind: {} {:?}", action.name, device)),
            segmented_option(text),
            RebindChip {
                action: action.name,
                device,
            },
            observe(on_rebind_chip),
        ));
        if armed {
            chip.insert(Selected);
        }
    });
}

/// How wide each device column of a Controls row is. Fixed so the two columns
/// line up down the tab whatever a row happens to be bound to.
const CHIP_WIDTH: f32 = 132.0;

/// Load the persisted settings once at startup and write them into the live
/// resources. A missing/corrupt store is a no-op (the resources keep their
/// defaults). Runs before the first `Update`, so nova_gameplay's apply systems
/// (gated on `resource_changed`) push the loaded values onto the engine on the
/// first frame.
pub(crate) fn load_persisted_settings(
    mut volume: ResMut<MasterVolume>,
    mut quality: ResMut<GraphicsQuality>,
    mut skin: ResMut<UiSkin>,
    mut monitor: ResMut<NovaOsMonitorSettings>,
    mut window_mode: ResMut<WindowModeSetting>,
    mut bindings: ResMut<InputBindings>,
) {
    let Some(saved) = load_settings() else {
        return;
    };
    *volume = MasterVolume(saved.master_volume.clamp(0.0, 1.0));
    *quality = saved.graphics_quality;
    *skin = saved.ui_skin;
    *monitor = saved.nova_os_monitor();
    *window_mode = saved.window_mode;
    // Before the first rig is built: the flight rig spawns with the player
    // ship, which is a scenario away, so a saved keybind is on the table by
    // the time anything reads it.
    bindings.apply_overrides(&saved.keybinds);
}

/// Put the chosen window mode on the primary window. Native only - see
/// [`WindowModeSetting`].
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn apply_window_mode(
    setting: Res<WindowModeSetting>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    use bevy::window::{MonitorSelection, WindowMode};

    if !setting.is_changed() {
        return;
    }
    let mode = match *setting {
        WindowModeSetting::Windowed => WindowMode::Windowed,
        WindowModeSetting::Borderless => {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        }
    };
    for mut window in &mut windows {
        if window.mode != mode {
            window.mode = mode;
        }
    }
}

/// Idle frames a settings value must hold steady before it is written to disk.
/// Debounces the volume slider, whose drag mutates `MasterVolume` every frame:
/// without this, one drag would trigger a full config write per frame. ~0.25s at
/// 60fps - imperceptible for a settings save, and it collapses a whole drag (or
/// a track-click, which emits no final `ValueChange`) into a single write.
pub(crate) const SETTINGS_SAVE_DEBOUNCE_FRAMES: u32 = 15;

/// Persist the settings a short beat after the player stops editing. Any change
/// (re)arms the debounce; the save fires once the value has held steady for
/// [`SETTINGS_SAVE_DEBOUNCE_FRAMES`]. The initial add (startup load /
/// `init_resource`) is skipped via `is_added`, so a launch that changes nothing
/// never arms the debounce and never rewrites the store. `Local` holds the idle
/// countdown: `None` = nothing pending, `Some(n)` = `n` idle frames so far.
pub(crate) fn persist_settings_on_change(
    settings: LiveSettings,
    mut pending: ResMut<PendingSettingsSave>,
) {
    if settings.edited() {
        // A fresh edit: (re)start the debounce, coalescing a drag's per-frame
        // changes into one pending save.
        pending.idle_frames = Some(0);
        return;
    }
    if let Some(frames) = pending.idle_frames {
        if frames + 1 >= SETTINGS_SAVE_DEBOUNCE_FRAMES {
            save_settings(&settings.snapshot());
            pending.idle_frames = None;
        } else {
            pending.idle_frames = Some(frames + 1);
        }
    }
}

/// Every resource the store holds, as one system parameter: the two systems
/// that write the file both need all of them, and a settings added to one and
/// not the other is how a value silently stops being saved.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct LiveSettings<'w> {
    volume: Res<'w, MasterVolume>,
    quality: Res<'w, GraphicsQuality>,
    skin: Res<'w, UiSkin>,
    monitor: Res<'w, NovaOsMonitorSettings>,
    window_mode: Res<'w, WindowModeSetting>,
    bindings: Res<'w, InputBindings>,
}

impl LiveSettings<'_> {
    /// Whether the player moved something this frame. The initial add (startup
    /// load / `init_resource`) does not count: a launch that changes nothing
    /// must not rewrite the store.
    fn edited(&self) -> bool {
        let moved = |changed: bool, added: bool| changed && !added;
        moved(self.volume.is_changed(), self.volume.is_added())
            || moved(self.quality.is_changed(), self.quality.is_added())
            || moved(self.skin.is_changed(), self.skin.is_added())
            || moved(self.monitor.is_changed(), self.monitor.is_added())
            || moved(self.window_mode.is_changed(), self.window_mode.is_added())
            || moved(self.bindings.is_changed(), self.bindings.is_added())
    }

    /// The persistable form of what is live right now.
    fn snapshot(&self) -> PersistedSettings {
        PersistedSettings::from_resources(
            *self.volume,
            *self.quality,
            *self.skin,
            *self.monitor,
            *self.window_mode,
            &self.bindings,
        )
    }
}

/// The debounce countdown, as a resource rather than a `Local` so
/// [`flush_settings_on_exit`] can see that a write is owed. `None` = nothing
/// pending, `Some(n)` = `n` idle frames so far.
#[derive(Resource, Default)]
pub(crate) struct PendingSettingsSave {
    idle_frames: Option<u32>,
}

/// Write an owed settings save before the process goes away.
///
/// The debounce is [`SETTINGS_SAVE_DEBOUNCE_FRAMES`] (~0.25s) and the Exit
/// button writes [`AppExit`] the same frame it is clicked, so a value edited
/// just before quitting is otherwise lost. Runs in `Last`, which the app
/// runner drains `AppExit` after.
pub(crate) fn flush_settings_on_exit(
    mut exits: MessageReader<AppExit>,
    settings: LiveSettings,
    mut pending: ResMut<PendingSettingsSave>,
) {
    if exits.is_empty() || pending.idle_frames.is_none() {
        return;
    }
    exits.clear();
    save_settings(&settings.snapshot());
    pending.idle_frames = None;
}

/// Mirror the volume slider's value onto [`MasterVolume`] as it is dragged.
/// bevy's `slider_self_update` (registered alongside this) commits the value
/// onto the slider's own `SliderValue`; this copies it to the resource, whose
/// change then drives the audio (`GlobalVolume` + the thruster loop) and the
/// save-on-change persistence. Guarded on [`VolumeSlider`] so it ignores any
/// other slider.
pub(crate) fn on_volume_slider_change(
    change: On<ValueChange<f32>>,
    is_volume: Query<(), With<VolumeSlider>>,
    mut volume: ResMut<MasterVolume>,
) {
    if is_volume.contains(change.source) {
        *volume = MasterVolume(change.value.clamp(0.0, 1.0));
    }
}

/// Keep the volume slider's percent label in sync with its value. The bar fill
/// is the shared `slider_track`, shown by nova_ui's `sync_slider_tracks` in
/// either skin - so this only owns the `NN%` text. Runs every frame;
/// there is at most one slider (main-menu or pause), and none while no settings
/// panel is open.
pub(crate) fn sync_volume_slider(
    sliders: Query<&SliderValue, With<VolumeSlider>>,
    mut labels: Query<&mut Text, With<VolumeLabel>>,
) {
    if let Ok(value) = sliders.single() {
        for mut text in &mut labels {
            text.0 = volume_label(value.0);
        }
    }
}

/// The controls that are not enhanced-input actions, and so are not in the
/// registry: raw `ButtonInput` chords read by the pause overlay. They stay
/// declared here until someone names them.
const FIXED_ROWS: &[(&str, &str, &str, &str)] = &[("SYSTEM", "Pause / Menu", "Esc", "Start")];

/// One read-only keybind row: the action on the left, the keyboard and gamepad
/// bindings on the right.
pub(crate) fn spawn_keybind_row(
    list: &mut ChildSpawnerCommands,
    action: &str,
    keyboard: &str,
    gamepad: &str,
) {
    list.spawn((
        Name::new(format!("Keybind: {action}")),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: UiRect::axes(px(2), px(3)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            UiText,
            Text::new(action.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
        ));
        row.spawn((
            UiText,
            Text::new(keyboard.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
            Node {
                width: px(CHIP_WIDTH),
                ..default()
            },
        ));
        row.spawn((
            UiText,
            Text::new(gamepad.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
            Node {
                width: px(CHIP_WIDTH),
                ..default()
            },
        ));
    });
}
