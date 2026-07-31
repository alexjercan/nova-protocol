//! The Settings panel: the shared body spawned by both the main menu and the
//! pause overlay, its live controls, and the persistence systems.

use bevy::{
    prelude::*,
    ui_widgets::{Activate, Slider, SliderRange, SliderStep, SliderValue, TrackClick, ValueChange},
};
use nova_gameplay::prelude::*;
use nova_ui::{
    prelude::UiSkin,
    theme,
    widget::{
        panel_header, segmented_container, segmented_option, separator, slider_track, ButtonValue,
        Selected,
    },
};

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

// The settings segmented rows use nova_ui's shared `segmented_container` +
// `segmented_option` (the same helpers the widget_zoo uses); the caller adds the
// `ButtonValue<T>` + `Selected` that `button_on_setting` drives.

/// Build the shared settings body (audio volume, graphics preset, read-only
/// keybind reference) under `list`. Used by BOTH the main-menu Settings overlay
/// and the pause-menu Settings overlay so the two entry points stay one modal
/// (user note 2026-07-16). Selection highlights are seeded from the current
/// resource values; presses are handled by the app-global
/// `button_on_setting::<T>` observers, so this builder spawns no observers.
pub(crate) fn build_settings_body(
    list: &mut ChildSpawnerCommands,
    volume: MasterVolume,
    quality: GraphicsQuality,
    skin: UiSkin,
) {
    // AUDIO - master volume as a draggable slider (bevy's headless `Slider`;
    // drag handling comes from `UiWidgetsPlugins` in DefaultPlugins, the value
    // is committed by `slider_self_update` and mirrored to `MasterVolume` by
    // `on_volume_slider_change`, both registered in the plugin).
    list.spawn(panel_header("Audio"));
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

    list.spawn(separator());

    // GRAPHICS - the quality preset. Each tier drives the combat juice today; the low-
    // end mode extends what Low/Medium skip.
    list.spawn(panel_header("Graphics"));
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

    list.spawn(separator());

    // CONTROLS - a read-only reference of the current bindings.
    list.spawn(panel_header("Controls"));
    let mut current_section = "";
    for entry in keybind_reference() {
        if entry.section != current_section {
            current_section = entry.section;
            list.spawn((
                Name::new(format!("Controls Section: {}", entry.section)),
                Text::new(entry.section),
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
        }
        spawn_keybind_row(list, entry);
    }

    list.spawn(separator());

    // INTERFACE - the UI skin choice. A segmented Phosphor|Hardware control
    // wired through `ButtonValue<UiSkin>` + the app-global
    // `button_on_setting::<UiSkin>` observer, exactly like GRAPHICS above.
    list.spawn(panel_header("Interface"));
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
) {
    let Some(saved) = load_settings() else {
        return;
    };
    *volume = MasterVolume(saved.master_volume.clamp(0.0, 1.0));
    *quality = saved.graphics_quality;
    *skin = saved.ui_skin;
    *monitor = saved.nova_os_monitor();
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
    volume: Res<MasterVolume>,
    quality: Res<GraphicsQuality>,
    skin: Res<UiSkin>,
    monitor: Res<NovaOsMonitorSettings>,
    mut idle_frames: Local<Option<u32>>,
) {
    let edited = (volume.is_changed() && !volume.is_added())
        || (quality.is_changed() && !quality.is_added())
        || (skin.is_changed() && !skin.is_added())
        || (monitor.is_changed() && !monitor.is_added());
    if edited {
        // A fresh edit: (re)start the debounce, coalescing a drag's per-frame
        // changes into one pending save.
        *idle_frames = Some(0);
        return;
    }
    if let Some(frames) = *idle_frames {
        if frames + 1 >= SETTINGS_SAVE_DEBOUNCE_FRAMES {
            save_settings(&PersistedSettings::from_resources(
                *volume, *quality, *skin, *monitor,
            ));
            *idle_frames = None;
        } else {
            *idle_frames = Some(frames + 1);
        }
    }
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

/// One read-only keybind row: the action on the left, the keyboard and gamepad
/// bindings on the right.
pub(crate) fn spawn_keybind_row(list: &mut ChildSpawnerCommands, entry: &KeybindEntry) {
    list.spawn((
        Name::new(format!("Keybind: {}", entry.action)),
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
            Text::new(entry.action),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
        ));
        row.spawn((
            Text::new(format!("{}   ·   {}", entry.keyboard, entry.gamepad)),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
        ));
    });
}
