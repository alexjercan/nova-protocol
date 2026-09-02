//! The Settings panel: the tab bar and body spawned by both the main menu and
//! the pause overlay, the rebind capture, and the persistence systems.
//!
//! The body is RECONCILED, not built once. Four tabs share one container and
//! [`refresh_settings_tab`] fills it from the live resources, so a rebind, a
//! tab press or a loaded store all reach the screen through the same path.

use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{
        observe, Activate, Slider, SliderRange, SliderStep, SliderValue, TrackClick, ValueChange,
    },
};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_hud::prelude::{KeyGlyphs, NovaHudAssets};
use nova_input::prelude::*;
use nova_os_ui::prelude::NovaOsMonitorSettings;
use nova_ship::prelude::{
    SpaceshipRailgunInputBinding, SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding,
    SpaceshipTurretInputBinding,
};
use nova_ui::{
    prelude::UiSkin,
    theme,
    widget::{
        panel_header, segmented_container, segmented_container_wrapping, segmented_option,
        segmented_option_fit, separator, slider_track, ButtonValue, Selected, UiText,
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

/// One row of the Audio tab: which mixer track its slider moves.
///
/// The tab is four of these, so adding a track is a variant plus its resource
/// rather than another hand-wired slider.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum VolumeChannel {
    /// Everything, through bevy's `GlobalVolume`.
    Master,
    /// UI chrome.
    Interface,
    /// Everything diegetic - the ship, the guns, the world.
    World,
    /// RESERVED: the slider moves `MusicVolume` and the store saves it, but no
    /// sound routes to that bus yet.
    Music,
}

impl VolumeChannel {
    /// The rows, top to bottom. Master first: it is the one every player
    /// reaches for.
    pub(crate) const ALL: [Self; 4] = [Self::Master, Self::Interface, Self::World, Self::Music];

    /// What the row reads.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Master => "Master",
            Self::Interface => "Interface",
            Self::World => "World",
            Self::Music => "Music",
        }
    }
}

/// One track's live level, `0.0..=1.0`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AudioLevels {
    master: f32,
    interface: f32,
    world: f32,
    music: f32,
}

impl AudioLevels {
    fn of(self, channel: VolumeChannel) -> f32 {
        match channel {
            VolumeChannel::Master => self.master,
            VolumeChannel::Interface => self.interface,
            VolumeChannel::World => self.world,
            VolumeChannel::Music => self.music,
        }
    }
}

/// A volume [`Slider`] entity (bevy's headless slider widget), tagged with the
/// track it moves so the change observer and the label sync can find it.
#[derive(Component)]
pub(crate) struct VolumeSlider(pub(crate) VolumeChannel);

/// The "72%" readout beside one volume slider.
#[derive(Component)]
pub(crate) struct VolumeLabel(pub(crate) VolumeChannel);

/// A mouse-sensitivity [`Slider`], tagged with the path it moves. Its value is
/// the PERCENTAGE; the raw engine gain behind it is what
/// [`on_sensitivity_slider_change`] writes.
#[derive(Component)]
pub(crate) struct SensitivitySlider(pub(crate) MousePath);

/// The "200%" readout beside one sensitivity slider.
#[derive(Component)]
pub(crate) struct SensitivityLabel(pub(crate) MousePath);

/// Format a whole percentage as its readout.
pub(crate) fn percent_label(percent: f32) -> String {
    format!("{}%", percent.round() as i32)
}

/// Format a linear volume factor as a whole-percent label.
pub(crate) fn volume_label(value: f32) -> String {
    percent_label(value.clamp(0.0, 1.0) * 100.0)
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

/// Which binding GROUP the Controls tab is showing.
///
/// The registry has eight groups and a panel has room for a dozen rows, so the
/// tab shows one group at a time behind its own bar rather than one scroll of
/// everything. Empty means "whichever group comes first", which is what a
/// fresh install and a group that was removed both resolve to.
#[derive(Resource, Default, PartialEq, Eq)]
pub(crate) struct SettingsControlsGroup(pub(crate) &'static str);

/// A Controls group-bar button: the group it opens.
#[derive(Component)]
pub(crate) struct ControlsGroupTab(pub(crate) &'static str);

/// The container [`refresh_settings_tab`] owns the children of. Both entry
/// points put it on their scrolling body, so one reconciler fills both.
#[derive(Component)]
pub(crate) struct SettingsTabBody;

/// The fixed strip between the tab bar and the scrolling body.
///
/// The Controls group bar and the rebind refusal live here rather than in the
/// body because the body SCROLLS: with both inside it, the group tabs left the
/// screen exactly when a long group gave a player the most rows to read, and
/// the refusal - the whole answer to "why did nothing happen when I pressed
/// that key" - was rendered below the fold, eight rows from the chip that
/// raised it.
#[derive(Component)]
pub(crate) struct SettingsControlsHeader;

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
    /// Whether any chip is waiting for a press.
    ///
    /// Read by `toggle_pause`, which must let an armed chip answer Escape first.
    pub(crate) fn is_armed(&self) -> bool {
        self.armed.is_some()
    }

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
    parent.spawn((
        Name::new("Settings Controls Header"),
        SettingsControlsHeader,
        Node {
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(px(6)),
            ..default()
        },
    ));
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

/// Open the clicked Controls group. The body is rebuilt from the resource, so
/// the `Selected` highlight moves with it rather than being placed by hand.
pub(crate) fn on_controls_group_tab(
    activate: On<Activate>,
    tabs: Query<&ControlsGroupTab>,
    mut open: ResMut<SettingsControlsGroup>,
    mut rebind: ResMut<PendingRebind>,
) {
    let Ok(tab) = tabs.get(activate.entity) else {
        return;
    };
    if open.0 == tab.0 {
        return;
    }
    open.0 = tab.0;
    // Leaving a group with a chip armed would leave the next key press
    // captured by a row nothing is showing.
    disarm(&mut rebind);
}

/// Whether [`refresh_settings_tab`] has anything to redraw.
///
/// Deliberately NOT armed by `MasterVolume`: the slider mutates it every frame
/// of a drag, and rebuilding the body under the pointer would drop the drag.
/// The other settings move their own `Selected` highlight through
/// `button_on_setting`, so they need no rebuild either.
pub(crate) fn settings_tab_dirty(
    active: Res<SettingsActiveTab>,
    group: Res<SettingsControlsGroup>,
    bindings: Res<InputBindings>,
    rebind: Res<PendingRebind>,
    spawned: Query<(), Added<SettingsTabBody>>,
) -> bool {
    active.is_changed()
        || group.is_changed()
        || bindings.is_changed()
        || rebind.is_changed()
        || !spawned.is_empty()
}

/// Fill every [`SettingsTabBody`] with the open tab.
#[expect(
    clippy::too_many_arguments,
    reason = "the reconciler draws every tab, so it reads every setting the panel shows"
)]
pub(crate) fn refresh_settings_tab(
    mut commands: Commands,
    bodies: Query<Entity, With<SettingsTabBody>>,
    headers: Query<Entity, With<SettingsControlsHeader>>,
    active: Res<SettingsActiveTab>,
    group: Res<SettingsControlsGroup>,
    volume: Res<MasterVolume>,
    interface_volume: Res<InterfaceVolume>,
    world_volume: Res<WorldVolume>,
    music_volume: Res<MusicVolume>,
    sensitivity: Res<MouseSensitivity>,
    quality: Res<GraphicsQuality>,
    skin: Res<UiSkin>,
    window_mode: Res<WindowModeSetting>,
    bindings: Res<InputBindings>,
    rebind: Res<PendingRebind>,
    // Absent on a bare menu rig that never ran asset loading, which is exactly
    // the text-chip fallback the keycap table is designed to degrade to.
    hud_assets: Option<Res<NovaHudAssets>>,
) {
    let glyphs = hud_assets.as_deref().map(|assets| &assets.key_glyphs);
    let levels = AudioLevels {
        master: volume.factor(),
        interface: interface_volume.factor(),
        world: world_volume.factor(),
        music: music_volume.factor(),
    };
    // The fixed strip above the body. Emptied on every tab so a group bar left
    // over from Controls cannot sit above the Audio page.
    for strip in &headers {
        commands.entity(strip).despawn_related::<Children>();
        if active.0 == SettingsTabKind::Controls {
            commands.entity(strip).with_children(|strip| {
                build_controls_header(strip, &bindings, &rebind, group.0, *skin);
            });
        }
    }
    for body in &bodies {
        commands.entity(body).despawn_related::<Children>();
        commands.entity(body).with_children(|list| match active.0 {
            SettingsTabKind::Audio => build_audio_tab(list, levels, *skin),
            SettingsTabKind::Graphics => build_graphics_tab(list, *quality, *window_mode, *skin),
            SettingsTabKind::Controls => {
                build_controls_tab(
                    list,
                    &bindings,
                    &rebind,
                    group.0,
                    glyphs,
                    (*sensitivity, *skin),
                );
            }
            SettingsTabKind::Interface => build_interface_tab(list, *skin),
        });
    }
}

/// AUDIO - one draggable slider per mixer track (bevy's headless `Slider`; drag
/// handling comes from `UiWidgetsPlugins` in DefaultPlugins, the value is
/// committed by `slider_self_update` and mirrored to its resource by
/// `on_volume_slider_change`, both registered in the plugin).
///
/// Master scales everything through bevy's `GlobalVolume`; Interface and World
/// are the engine's two live buses. Music is RESERVED - the slider and the
/// saved value are here so the surface and the store do not need a format break
/// when music lands.
fn build_audio_tab(list: &mut ChildSpawnerCommands, levels: AudioLevels, skin: UiSkin) {
    for channel in VolumeChannel::ALL {
        build_volume_row(list, channel, levels.of(channel), skin);
    }
}

/// One track's row: its name, its slider, and its percent readout.
fn build_volume_row(
    list: &mut ChildSpawnerCommands,
    channel: VolumeChannel,
    value: f32,
    skin: UiSkin,
) {
    build_slider_row(
        list,
        SliderRow {
            name: format!("{} Volume", channel.label()),
            label: channel.label().to_string(),
            value,
            range: (0.0, 1.0),
            step: VOLUME_STEP,
            readout: volume_label(value),
        },
        VolumeSlider(channel),
        VolumeLabel(channel),
        skin,
    );
}

/// One settings slider row, independent of what it moves.
///
/// The audio tracks and the MOUSE sensitivities are the SAME widget over
/// different numbers - a ticked track and a live whole-percent readout - so
/// they are built from one description rather than two hand-kept copies that
/// can drift apart.
struct SliderRow {
    /// The `Name` prefix every entity in the row carries, e.g. `Master Volume`.
    name: String,
    /// The text down the left of the row.
    label: String,
    /// Where the handle sits, in the slider's own units.
    value: f32,
    /// The slider's two ends, in the same units.
    range: (f32, f32),
    /// One detent: the arrow-key step, and the spacing the drag cue ticks at.
    step: f32,
    /// The percent readout beside the track.
    readout: String,
}

/// Spawn one [`SliderRow`], carrying the markers that say which setting it
/// moves - one on the slider (read by the change observer) and one on the
/// readout (read by the label sync).
fn build_slider_row(
    list: &mut ChildSpawnerCommands,
    row: SliderRow,
    slider: impl Bundle,
    readout: impl Bundle,
    skin: UiSkin,
) {
    let name = row.name;
    list.spawn((
        Name::new(format!("{name} Row")),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            margin: UiRect::vertical(px(4)),
            ..default()
        },
    ))
    .with_children(|parent| {
        parent.spawn((
            Name::new(format!("{name} Label")),
            UiText,
            Text::new(row.label),
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
        parent
            .spawn(Node {
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|cell| {
                cell.spawn((
                    Name::new(format!("{name} Slider Track")),
                    slider,
                    Slider {
                        track_click: TrackClick::Snap,
                        ..default()
                    },
                    SliderValue(row.value),
                    SliderRange::new(row.range.0, row.range.1),
                    SliderStep(row.step),
                    slider_track(row.value, skin),
                ));
            });
        parent.spawn((
            Name::new(format!("{name} Readout")),
            readout,
            UiText,
            Text::new(row.readout),
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

/// The group bar and the refusal, in the strip that does NOT scroll.
///
/// Split out of [`build_controls_tab`] so the two things a player needs while
/// they work - which group they are on, and why a press was refused - stay on
/// screen however far down the rows they are.
fn build_controls_header(
    strip: &mut ChildSpawnerCommands,
    bindings: &InputBindings,
    rebind: &PendingRebind,
    open_group: &str,
    skin: UiSkin,
) {
    let groups = controls_groups(bindings);
    let Some(open) = open_group_of(&groups, open_group) else {
        return;
    };
    strip
        .spawn((
            Name::new("Controls Group Bar"),
            segmented_container_wrapping(skin),
        ))
        .with_children(|bar| {
            for group in &groups {
                let mut button = bar.spawn((
                    Name::new(format!("Controls Group: {group}")),
                    segmented_option_fit(group),
                    ControlsGroupTab(group),
                    observe(on_controls_group_tab),
                ));
                if *group == open {
                    button.insert(Selected);
                }
            }
        });
    if let Some(reason) = &rebind.refusal {
        strip.spawn((
            Name::new("Rebind Refusal"),
            UiText,
            Text::new(reason.clone()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::AMBER_NOVA),
            Node {
                margin: UiRect::top(px(6)),
                ..default()
            },
        ));
    }
}

/// Every group the Controls tab can open: the table's own, plus the groups that
/// are not bindings at all - the mouse sensitivities and the fixed chords.
fn controls_groups(bindings: &InputBindings) -> Vec<&'static str> {
    let mut groups = bindings.groups();
    for group in [MOUSE_GROUP]
        .into_iter()
        .chain(FIXED_ROWS.iter().map(|(g, ..)| *g))
    {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
}

/// The Controls group that holds the mouse sensitivities.
///
/// Not a registry group: nothing in it is a binding, so no action carries the
/// name and the tab has to know it the way it knows [`FIXED_ROWS`]'s `SYSTEM`.
pub(crate) const MOUSE_GROUP: &str = "MOUSE";

/// A group the table no longer carries falls back to the first, so a store
/// written before a group was renamed opens on something rather than blank.
fn open_group_of(groups: &[&'static str], open_group: &str) -> Option<&'static str> {
    groups
        .iter()
        .copied()
        .find(|group| *group == open_group)
        .or_else(|| groups.first().copied())
}

/// CONTROLS - one GROUP at a time: the binding rows off the LIVE table, then
/// the chords in that group that are not actions at all.
///
/// A shadow row (`radar_clear`) is absent: it moves with the action it
/// follows, so showing it would offer a second way to break one gesture.
///
/// [`MOUSE_GROUP`] is the one page with no bindings on it: three sensitivity
/// sliders, and NO reset button. Reset Defaults is a keybinding operation, and
/// a page with nothing to rebind must not offer it - it would read as "put the
/// sensitivities back", which is not what it does.
fn build_controls_tab(
    list: &mut ChildSpawnerCommands,
    bindings: &InputBindings,
    rebind: &PendingRebind,
    open_group: &str,
    glyphs: Option<&KeyGlyphs>,
    mouse: (MouseSensitivity, UiSkin),
) {
    let groups = controls_groups(bindings);
    let Some(open) = open_group_of(&groups, open_group) else {
        return;
    };

    if open == MOUSE_GROUP {
        let (sensitivity, skin) = mouse;
        for path in MousePath::ALL {
            build_sensitivity_row(list, path, sensitivity.percent(path), skin);
        }
        return;
    }

    for action in bindings.rows().filter(|action| action.group == open) {
        spawn_rebind_row(list, action, rebind, glyphs);
    }
    for (_, label, keyboard, gamepad) in FIXED_ROWS.iter().filter(|(group, ..)| *group == open) {
        spawn_keybind_row(list, label, keyboard, gamepad, glyphs);
    }

    list.spawn(separator());
    list.spawn((
        Name::new("Reset Bindings"),
        segmented_option("Reset Defaults"),
        ResetBindings,
        observe(on_reset_bindings),
    ));
}

/// One mouse path's row, in the Audio slider's own presentation. The slider
/// speaks PERCENTAGES - each path's own `100%` baseline - because that is the
/// only reading of three gains two orders of magnitude apart a player can
/// compare; the raw engine value is what the observer stores.
fn build_sensitivity_row(
    list: &mut ChildSpawnerCommands,
    path: MousePath,
    percent: f32,
    skin: UiSkin,
) {
    let range = path.range();
    build_slider_row(
        list,
        SliderRow {
            name: path.label().to_string(),
            label: path.label().to_string(),
            value: percent,
            range: (MouseSensitivityRange::MIN_PERCENT, range.max_percent),
            step: range.percent_step(),
            readout: percent_label(percent),
        },
        SensitivitySlider(path),
        SensitivityLabel(path),
        skin,
    );
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
    panels: Query<&Visibility, Or<(With<SettingsPanel>, With<PauseSettingsPanel>)>>,
    sections: SectionBindings,
) {
    // A capture belongs to the row that armed it, and this system is ungated by
    // menu state on purpose - the pause overlay shows the same body. So the arm
    // has to be dropped HERE when the surface goes away: Back and the pause
    // toggle only flip `Visibility`, and a state exit despawns the panel
    // outright, so neither runs a handler that could lower it. An arm that
    // outlived its own screen would silently take the next key the player
    // pressed anywhere, write it to the table, and persist it.
    if !panels.iter().any(|vis| *vis == Visibility::Visible) {
        disarm(&mut rebind);
        return;
    }
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
        let reason = format!(
            "{} is already bound to {}",
            source.readout_label(),
            taken_by.label
        );
        rebind.refusal = Some(reason);
        return;
    }
    if let Some(taken_by) = section_conflict(bindings.get(action), source, &sections) {
        rebind.refusal = Some(format!(
            "{} is already bound to {taken_by}",
            source.readout_label()
        ));
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

/// Every live section that carries a player trigger, with the stable id the
/// refusal names it by.
type SectionBindings<'w, 's> = Query<
    'w,
    's,
    (
        &'static EntityId,
        Option<&'static SpaceshipThrusterInputBinding>,
        Option<&'static SpaceshipTurretInputBinding>,
        Option<&'static SpaceshipTorpedoInputBinding>,
        Option<&'static SpaceshipRailgunInputBinding>,
    ),
    Or<(
        With<SpaceshipThrusterInputBinding>,
        With<SpaceshipTurretInputBinding>,
        With<SpaceshipTorpedoInputBinding>,
        With<SpaceshipRailgunInputBinding>,
    )>,
>;

/// What a LIVE ship section already spends `source` on.
///
/// A section's trigger is not a registry action - it lives on the section
/// entity, authored per ship - so [`InputBindings::conflict_for`] cannot see
/// it and the guard was one-directional: the ship viewer refuses a section
/// binding a flight verb holds, and nothing refused a flight verb landing on
/// a section's trigger. Every base scenario arms its turrets on the right
/// trigger, so Main Drive could be bound to it and one pull would burn AND
/// fire.
///
/// Only actions that can be live WITH the ship are checked, for the same
/// reason `conflict_for` checks contexts: a NOVA OS verb on the turret
/// trigger is not a collision, because the flight rig is down while the
/// computer holds the screen.
fn section_conflict(
    action: Option<&ActionBinding>,
    source: InputSource,
    sections: &SectionBindings,
) -> Option<String> {
    if !action?.context.overlaps(ActionContext::Flight) {
        return None;
    }
    sections
        .iter()
        .find(|(_, thruster, turret, torpedo, railgun)| {
            let held = |binds: Option<&Vec<InputSource>>| {
                binds.is_some_and(|binds| binds.contains(&source))
            };
            held(thruster.map(|b| &b.0))
                || held(turret.map(|b| &b.0))
                || held(torpedo.map(|b| &b.0))
                || held(railgun.map(|b| &b.0))
        })
        .map(|(id, ..)| format!("the ship's {} section", id.0))
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
    glyphs: Option<&KeyGlyphs>,
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
            &action.keyboard_chips(),
            rebind,
            glyphs,
        );
        spawn_chip(
            row,
            action,
            RebindDevice::Pad,
            !action.gamepad.is_empty(),
            &action.gamepad_chips(),
            rebind,
            glyphs,
        );
    });
}

/// One device column of a rebind row: a button when the action holds a source
/// there, plain text when it does not.
///
/// The button carries NO label of its own - [`spawn_binding_chips`] fills it
/// with keycap pictures instead, which is why it is built from an empty
/// `segmented_option`.
fn spawn_chip(
    row: &mut ChildSpawnerCommands,
    action: &ActionBinding,
    device: RebindDevice,
    rebindable: bool,
    chips: &[BindingChip],
    rebind: &PendingRebind,
    glyphs: Option<&KeyGlyphs>,
) {
    let armed = rebind.armed_on(action.name, device);
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
        // The SAME chip, wearing the disabled face: a column an action cannot
        // hold a button in still reads as a chip in the column, and the paint
        // is what says it will not take a press. `InteractionDisabled` also
        // stops it lighting under the pointer, which a bare frame could not.
        cell.with_children(|cell| {
            cell.spawn((
                Name::new(format!("Controls Fixed: {} {:?}", action.name, device)),
                segmented_option(""),
                InteractionDisabled,
            ))
            .with_children(|slot| {
                spawn_binding_chips(
                    slot,
                    chips,
                    glyphs,
                    theme::PHOSPHOR_MUTED,
                    theme::PHOSPHOR_MUTED,
                );
            });
        });
        return;
    }
    cell.with_children(|cell| {
        let mut chip = cell.spawn((
            Name::new(format!("Rebind: {} {:?}", action.name, device)),
            segmented_option(if armed { device.prompt() } else { "" }),
            RebindChip {
                action: action.name,
                device,
            },
            observe(on_rebind_chip),
        ));
        if armed {
            chip.insert(Selected);
        } else {
            chip.with_children(|slot| {
                spawn_binding_chips(slot, chips, glyphs, theme::SCREEN_TEXT, Color::WHITE);
            });
        }
    });
}

/// Draw one binding column: a KEYCAP per chip where the pack has art for it,
/// the chip's own text where it does not.
///
/// A keycap is sized off its measured cap rect ([`KeyCap::node`]), so the wide
/// ones (Space, Shift, Tab) keep their proportions instead of being squashed
/// into a square.
fn spawn_binding_chips(
    slot: &mut ChildSpawnerCommands,
    chips: &[BindingChip],
    glyphs: Option<&KeyGlyphs>,
    text_color: Color,
    glyph_tint: Color,
) {
    if chips.is_empty() {
        slot.spawn((
            UiText,
            Text::new("Unbound"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(text_color),
        ));
        return;
    }
    for chip in chips {
        let cap =
            glyphs.and_then(|glyphs| chip.glyph.as_deref().and_then(|label| glyphs.get(label)));
        match cap {
            Some(cap) => {
                let (mut image, node) = cap.node(CHIP_GLYPH_PX);
                // The PICTURE carries the disabled paint too, not only the text
                // fallback. While the tint reached the text branch alone, a row
                // that cannot be changed - Escape, Aim - drew a full-brightness
                // keycap indistinguishable from the rebindable row under it, and
                // the Pause row disagreed with itself: a bright Esc cap beside
                // greyed-out `Start` text.
                image.color = glyph_tint;
                slot.spawn((Name::new(format!("Keycap: {}", chip.text)), image, node));
            }
            None => {
                slot.spawn((
                    UiText,
                    Text::new(chip.text.clone()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(text_color),
                ));
            }
        }
    }
}

/// How wide each device column of a Controls row is. Fixed so the two columns
/// line up down the tab whatever a row happens to be bound to.
const CHIP_WIDTH: f32 = 132.0;

/// How tall a keycap picture is drawn in a Controls chip. Width follows the
/// art, so this is the ONE number that sets the row rhythm.
const CHIP_GLYPH_PX: f32 = 20.0;

/// Load the persisted settings once at startup and write them into the live
/// resources. A missing/corrupt store is a no-op (the resources keep their
/// defaults). Runs before the first `Update`, so nova_gameplay's apply systems
/// (gated on `resource_changed`) push the loaded values onto the engine on the
/// first frame.
pub(crate) fn load_persisted_settings(
    mut volume: ResMut<MasterVolume>,
    mut interface_volume: ResMut<InterfaceVolume>,
    mut world_volume: ResMut<WorldVolume>,
    mut music_volume: ResMut<MusicVolume>,
    mut sensitivity: ResMut<MouseSensitivity>,
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
    *interface_volume = InterfaceVolume(saved.interface_volume.clamp(0.0, 1.0));
    *world_volume = WorldVolume(saved.world_volume.clamp(0.0, 1.0));
    *music_volume = MusicVolume(saved.music_volume.clamp(0.0, 1.0));
    *sensitivity = saved.mouse_sensitivity();
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
    interface_volume: Res<'w, InterfaceVolume>,
    world_volume: Res<'w, WorldVolume>,
    music_volume: Res<'w, MusicVolume>,
    sensitivity: Res<'w, MouseSensitivity>,
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
            || moved(
                self.interface_volume.is_changed(),
                self.interface_volume.is_added(),
            )
            || moved(self.world_volume.is_changed(), self.world_volume.is_added())
            || moved(self.music_volume.is_changed(), self.music_volume.is_added())
            || moved(self.sensitivity.is_changed(), self.sensitivity.is_added())
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
            *self.interface_volume,
            *self.world_volume,
            *self.music_volume,
            *self.sensitivity,
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

/// Mirror a volume slider's value onto its track's resource as it is dragged.
/// bevy's `slider_self_update` (registered alongside this) commits the value
/// onto the slider's own `SliderValue`; this copies it to the resource, whose
/// change then drives the audio and the save-on-change persistence. Guarded on
/// [`VolumeSlider`], whose channel says which resource to write, so it ignores
/// any other slider.
/// One notch of a volume slider: the arrow-key step, and the spacing the drag
/// cue ticks at.
const VOLUME_STEP: f32 = 0.05;

/// Which `step`-wide notch a value falls in. The drag cue's whole detent.
fn notch(value: f32, step: f32) -> i32 {
    (value / step).round() as i32
}

pub(crate) fn on_volume_slider_change(
    change: On<ValueChange<f32>>,
    sliders: Query<&VolumeSlider>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut master: ResMut<MasterVolume>,
    mut interface: ResMut<InterfaceVolume>,
    mut world: ResMut<WorldVolume>,
    mut music: ResMut<MusicVolume>,
) {
    let Ok(slider) = sliders.get(change.source) else {
        return;
    };
    let value = change.value.clamp(0.0, 1.0);
    let was = match slider.0 {
        VolumeChannel::Master => master.0,
        VolumeChannel::Interface => interface.0,
        VolumeChannel::World => world.0,
        VolumeChannel::Music => music.0,
    };
    match slider.0 {
        VolumeChannel::Master => *master = MasterVolume(value),
        VolumeChannel::Interface => *interface = InterfaceVolume(value),
        VolumeChannel::World => *world = WorldVolume(value),
        VolumeChannel::Music => *music = MusicVolume(value),
    }

    // The detent, and it has to be counted here. `SliderStep` governs the
    // arrow keys and a click on the track; the DRAG path rounds on
    // `SliderPrecision`, which this slider does not carry and nothing in the
    // repo sets - so a drag emits a raw float every frame it moves and a
    // straight `value != was` would tick once per FRAME. The notch index is
    // the comparison the old one only looked like. The volume itself stays
    // continuous: the tick is the detent, not the value.
    if notch(value, VOLUME_STEP) == notch(was, VOLUME_STEP) {
        return;
    }
    play_detent_tick(&mut commands, bank.as_deref());
}

/// Mirror a sensitivity slider's value onto [`MouseSensitivity`] as it is
/// dragged, and tick once per detent crossed - the volume slider's contract, on
/// the mouse gains.
///
/// The slider reports a PERCENTAGE of the path's own baseline; the resource
/// stores the raw engine gain, and `apply_mouse_sensitivity` (nova_input) puts
/// it on the live bindings - which is what makes a slider moved from the pause
/// overlay reach a ship that is already flying, with no respawn or reload.
pub(crate) fn on_sensitivity_slider_change(
    change: On<ValueChange<f32>>,
    sliders: Query<&SensitivitySlider>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut sensitivity: ResMut<MouseSensitivity>,
) {
    let Ok(slider) = sliders.get(change.source) else {
        return;
    };
    let path = slider.0;
    let range = path.range();
    let was = sensitivity.percent(path);
    let value = range.clamp_percent(change.value);
    sensitivity.set_percent(path, value);

    // The same detent rule the volume sliders use: a DRAG emits a raw float
    // every frame it moves, so the tick has to count notches rather than
    // compare values.
    let step = range.percent_step();
    if notch(value, step) == notch(was, step) {
        return;
    }
    play_detent_tick(&mut commands, bank.as_deref());
}

/// The UI cue one crossed slider detent makes. Shared so the audio and mouse
/// sliders cannot end up ticking differently.
fn play_detent_tick(commands: &mut Commands, bank: Option<&SoundBank<UiSfx>>) {
    if let Some(bank) = bank {
        commands.play_sfx(
            bank.get(UiSfx::UiTick),
            AudioRoute::Interface,
            UI_TICK_VOLUME,
        );
    }
}

/// Keep each volume slider's percent readout in sync with its own value. The
/// bar fill is the shared `slider_track`, shown by nova_ui's
/// `sync_slider_tracks` in either skin - so this only owns the `NN%` text. Runs
/// every frame; there is at most one Audio tab open (main-menu or pause), and
/// none while no settings panel is open.
pub(crate) fn sync_volume_slider(
    sliders: Query<(&SliderValue, &VolumeSlider)>,
    mut labels: Query<(&mut Text, &VolumeLabel)>,
) {
    for (value, slider) in &sliders {
        for (mut text, label) in &mut labels {
            if label.0 == slider.0 {
                text.0 = volume_label(value.0);
            }
        }
    }
}

/// Keep each sensitivity slider's percent readout in sync with its own value,
/// exactly as [`sync_volume_slider`] does for the mixer tracks.
pub(crate) fn sync_sensitivity_slider(
    sliders: Query<(&SliderValue, &SensitivitySlider)>,
    mut labels: Query<(&mut Text, &SensitivityLabel)>,
) {
    for (value, slider) in &sliders {
        for (mut text, label) in &mut labels {
            if label.0 == slider.0 {
                text.0 = percent_label(value.0);
            }
        }
    }
}

/// The controls that are not enhanced-input actions, and so are not in the
/// registry: raw `ButtonInput` chords read by the pause overlay. They stay
/// declared here until someone names them.
const FIXED_ROWS: &[(&str, &str, &str, &str)] = &[("SYSTEM", "Pause / Menu", "Esc", "Start")];

/// One read-only keybind row: the action on the left, the keyboard and gamepad
/// bindings on the right, in the same two columns a rebindable row uses so the
/// fixed chords line up with the ones above them.
pub(crate) fn spawn_keybind_row(
    list: &mut ChildSpawnerCommands,
    action: &str,
    keyboard: &str,
    gamepad: &str,
    glyphs: Option<&KeyGlyphs>,
) {
    list.spawn((
        Name::new(format!("Keybind: {action}")),
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
            Text::new(action.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                ..default()
            },
        ));
        for (device, column) in [("Desk", keyboard), ("Pad", gamepad)] {
            // A fixed chord is spelled, not bound, so it has no `InputSource`
            // to label - the spelling IS the keycap key, which is why `Esc`
            // sits in the glyph table beside `Escape`.
            let chip = BindingChip {
                text: column.to_string(),
                glyph: Some(column.to_string()),
            };
            row.spawn((
                Name::new(format!("Controls Cell: {action} {device}")),
                Node {
                    width: px(CHIP_WIDTH),
                    ..default()
                },
            ))
            .with_children(|cell| {
                cell.spawn((
                    Name::new(format!("Controls Fixed: {action} {device}")),
                    segmented_option(""),
                    InteractionDisabled,
                ))
                .with_children(|slot| {
                    spawn_binding_chips(
                        slot,
                        &[chip],
                        glyphs,
                        theme::PHOSPHOR_MUTED,
                        theme::PHOSPHOR_MUTED,
                    );
                });
            });
        }
    });
}
