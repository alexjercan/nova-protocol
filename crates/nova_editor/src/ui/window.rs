//! Floating windows: the layer they stand on, the bar you drag them by, and
//! the colour picker that is the first thing to live in one.
//!
//! A window is for an edit that needs ROOM - more than a 240px inspector row
//! can hold - and it is still an edit of the node the Inspector is on. It is
//! not a second view of the document: a window BELONGS to the row it was
//! opened from, so it follows what that row says and goes away with it.
//!
//! There is one at a time. The stage is what the builder came to look at, and
//! a screen of stacked panels is the generic editor this one is not.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        observe, Activate, Button, Slider, SliderRange, SliderStep, SliderValue, TrackClick,
        ValueChange,
    },
};
use nova_ui::{
    prelude::{panel, slider_track, themed_button, UiSkin, UiText},
    theme,
};

use crate::{
    bundle::ask_to_open,
    config::EditorSays,
    inspect::{colour_text, write_field},
    node::reset_document,
    ui::{
        inspector::{EditTargets, InspectorField, InspectorSwatch},
        menu::back_to_main_menu,
    },
};

/// Window width. The inspector's own width, so a picker reads as the row it
/// came from rather than as a second panel with its own ideas.
const WINDOW_W: f32 = 240.0;
/// How far off the right edge a fresh window stands, clear of the Inspector.
const RIGHT_MARGIN: f32 = 264.0;
/// Where a fresh window's top edge sits, under the top bar.
const TOP_MARGIN: f32 = 96.0;
/// How much of a window must stay on screen when it is dragged: enough to
/// grab the bar again.
const KEEP_ON_SCREEN: f32 = 48.0;

/// The layer every floating window stands on: screen-sized, transparent, and
/// deaf to the pointer, so the stage behind it is still buildable.
#[derive(Component)]
pub(crate) struct EditorWindowLayer;

/// One floating window.
#[derive(Component)]
pub(crate) struct EditorWindow;

/// The bar a window is dragged by, and the window it drags.
#[derive(Component)]
pub(crate) struct WindowTitleBar {
    window: Entity,
}

/// A window's close button, and what it closes.
#[derive(Component)]
pub(crate) struct WindowClose {
    window: Entity,
}

/// The colour picker: which field it writes to.
///
/// The field is the SAME one the row's text box holds, so the two are one
/// edit made two ways rather than two paths into the config.
#[derive(Component, Clone)]
pub(crate) struct ColourWindow {
    field: InspectorField,
}

/// One channel of a colour.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ColourChannel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl ColourChannel {
    /// The one-letter name the row wears.
    fn label(self) -> &'static str {
        match self {
            Self::Red => "R",
            Self::Green => "G",
            Self::Blue => "B",
            Self::Alpha => "A",
        }
    }

    /// This channel of `colour`, in `[0, 1]`.
    fn of(self, colour: Color) -> f32 {
        let srgba = Srgba::from(colour);
        match self {
            Self::Red => srgba.red,
            Self::Green => srgba.green,
            Self::Blue => srgba.blue,
            Self::Alpha => srgba.alpha,
        }
    }

    /// `colour` with this channel set to `value`.
    fn set(self, colour: Color, value: f32) -> Color {
        let mut srgba = Srgba::from(colour);
        let value = value.clamp(0.0, 1.0);
        match self {
            Self::Red => srgba.red = value,
            Self::Green => srgba.green = value,
            Self::Blue => srgba.blue = value,
            Self::Alpha => srgba.alpha = value,
        }
        Color::from(srgba)
    }
}

/// One channel's slider, and the window it belongs to.
#[derive(Component, Clone, Copy)]
pub(crate) struct ColourSlider {
    window: Entity,
    channel: ColourChannel,
}

/// A channel's number, beside its slider.
#[derive(Component, Clone, Copy)]
pub(crate) struct ChannelReadout {
    window: Entity,
    channel: ColourChannel,
}

/// The big block of the colour being picked.
#[derive(Component, Clone, Copy)]
pub(crate) struct ColourPreview {
    window: Entity,
}

/// The hex under the preview: the same text the row's box holds, so the two
/// widgets cannot disagree about what is being edited.
#[derive(Component, Clone, Copy)]
pub(crate) struct ColourReadout {
    window: Entity,
}

/// The layer, built empty. Windows are spawned into it as they are opened.
pub(crate) fn window_layer() -> impl Bundle {
    (
        Name::new("Editor Window Layer"),
        EditorWindowLayer,
        // Deaf: the layer covers the whole screen, and the stage under it is
        // still where the building happens. The WINDOWS block the pointer,
        // which is the panel's own doing, not the layer's.
        Pickable {
            should_block_lower: false,
            is_hoverable: false,
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        // Above the rail and the inspector: a window a panel could hide would
        // be a window nobody opened.
        GlobalZIndex(30),
    )
}

/// A verb that throws the document away, held back until the builder says yes.
///
/// On the menu ROW, not on the button that carries it out: the row asks, the
/// window's own button is what actually runs the verb.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DestructiveVerb {
    /// File > New Scenario.
    NewScenario,
    /// File > Open, which replaces what is on the stage with what is on disk.
    Open,
    /// File > Back to Main Menu, which ends the session and the document.
    MainMenu,
}

impl DestructiveVerb {
    /// The window's title.
    fn title(self) -> &'static str {
        match self {
            Self::NewScenario => "NEW SCENARIO",
            Self::Open => "OPEN",
            Self::MainMenu => "BACK TO MAIN MENU",
        }
    }

    /// What is about to be lost, said plainly. There is no undo and no
    /// autosave, so the sentence has to carry the whole warning.
    fn question(self) -> &'static str {
        match self {
            Self::NewScenario => "This throws away everything on the stage and starts an empty scenario. There is no undo.",
            Self::Open => "This replaces everything on the stage with the saved scenario. There is no undo.",
            Self::MainMenu => "Leaving ends the session. Anything not saved goes with it.",
        }
    }

    /// The label on the button that goes through with it. It names the VERB
    /// rather than saying "OK", so a builder reading only the buttons still
    /// knows which one is the destructive one.
    fn confirm(self) -> &'static str {
        match self {
            Self::NewScenario => "Discard and start over",
            Self::Open => "Discard and open",
            Self::MainMenu => "Discard and leave",
        }
    }
}

/// The window standing in front of a destructive verb.
#[derive(Component)]
pub(crate) struct ConfirmWindow;

/// Either of the confirm window's two answers: both close it.
#[derive(Component)]
pub(crate) struct ConfirmAnswer;

/// Confirm window width. Wider than the picker: this one is a sentence, and a
/// warning that wraps four times reads as fine print.
const CONFIRM_W: f32 = 360.0;

/// Put the question up instead of doing it.
///
/// One observer for all three verbs, keyed on the row's own
/// [`DestructiveVerb`]. The button inside the window carries the real verb's
/// observer, so there is no second copy of what any of them do.
pub(crate) fn on_destructive_item(
    activate: On<Activate>,
    rows: Query<&DestructiveVerb>,
    mut commands: Commands,
    skin: Res<UiSkin>,
    layer: Option<Single<Entity, With<EditorWindowLayer>>>,
    open: Query<(), With<ConfirmWindow>>,
    screen: Option<Single<&Window>>,
) {
    let Ok(verb) = rows.get(activate.entity) else {
        return;
    };
    let Some(layer) = layer else {
        return;
    };
    if !open.is_empty() {
        return;
    }
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let at = Vec2::new(((size.x - CONFIRM_W) * 0.5).max(8.0), TOP_MARGIN);
    commands
        .entity(*layer)
        .with_children(|layer| spawn_confirm_window(layer, *verb, at, *skin));
}

/// The question, its two answers, and the bar you can drag it by.
fn spawn_confirm_window(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    verb: DestructiveVerb,
    at: Vec2,
    skin: UiSkin,
) {
    let mut frame = layer.spawn((
        Name::new("Confirm Window"),
        EditorWindow,
        ConfirmWindow,
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x),
            top: px(at.y),
            width: px(CONFIRM_W),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        panel(skin),
    ));
    let window = frame.id();
    frame.with_children(|frame| {
        frame
            .spawn((
                Name::new("Confirm Window Bar"),
                WindowTitleBar { window },
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(8),
                    padding: UiRect::axes(px(10), px(6)),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    ..default()
                },
                BorderColor::all(theme::PHOSPHOR.with_alpha(0.16)),
                observe(on_window_drag),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Name::new("Confirm Window Title"),
                    UiText,
                    Text::new(verb.title()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::AMBER_NOVA),
                ));
            });
        frame
            .spawn((
                Name::new("Confirm Window Body"),
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(10),
                    padding: UiRect::all(px(10)),
                    ..default()
                },
            ))
            .with_children(|body| {
                body.spawn((
                    Name::new("Confirm Window Question"),
                    UiText,
                    Text::new(verb.question()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR),
                ));
                body.spawn((
                    Name::new("Confirm Window Answers"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        ..default()
                    },
                ))
                .with_children(|answers| {
                    // Keep editing FIRST and named for what it does. The safe
                    // answer is the one a builder should be able to reach
                    // without reading, and "Cancel" against a warning is
                    // ambiguous about which thing it cancels.
                    // A slot each, because `themed_button` is percent(100)
                    // wide - the growing is the slot's job, and the marker and
                    // the observer belong on the BUTTON, which is what emits
                    // the press.
                    answers
                        .spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|slot| {
                            slot.spawn((
                                Name::new("Confirm Keep Button"),
                                ConfirmAnswer,
                                themed_button("Keep editing"),
                            ));
                        });
                    answers
                        .spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|slot| {
                            let mut go = slot.spawn((
                                Name::new("Confirm Discard Button"),
                                ConfirmAnswer,
                                themed_button(verb.confirm()),
                            ));
                            match verb {
                                DestructiveVerb::NewScenario => go.observe(reset_document),
                                DestructiveVerb::Open => go.observe(ask_to_open),
                                DestructiveVerb::MainMenu => go.observe(back_to_main_menu),
                            };
                        });
                });
            });
    });
}

/// Take the question down, whichever answer was pressed.
///
/// Central, so the verb's own observer never has to know it was asked about.
pub(crate) fn close_confirm_window(
    activate: On<Activate>,
    answers: Query<(), With<ConfirmAnswer>>,
    windows: Query<Entity, With<ConfirmWindow>>,
    mut commands: Commands,
) {
    if !answers.contains(activate.entity) {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
}

/// Open the colour picker on the swatch that was clicked./// Open the colour picker on the swatch that was clicked.
///
/// The colour it opens on is the one the SWATCH is painted, which is the value
/// the Inspector last read off the document - so the picker never has to walk
/// the config itself.
pub(crate) fn on_open_colour_window(
    activate: On<Activate>,
    mut commands: Commands,
    skin: Res<UiSkin>,
    swatches: Query<(&InspectorField, &InspectorSwatch, &BackgroundColor)>,
    layer: Option<Single<Entity, With<EditorWindowLayer>>>,
    open: Query<(Entity, &ColourWindow)>,
    screen: Option<Single<&Window>>,
) {
    let Ok((field, swatch, painted)) = swatches.get(activate.entity) else {
        return;
    };
    let Some(layer) = layer else {
        return;
    };
    // One at a time. A second click on the SAME swatch puts the picker away
    // again - the swatch is the only control the row has, so it has to be
    // both the way in and the way out.
    let mut reopening = false;
    for (window, open) in &open {
        reopening |= open.field == *field;
        commands.entity(window).despawn();
    }
    if reopening {
        return;
    }
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let left = (size.x - RIGHT_MARGIN - WINDOW_W).max(8.0);
    commands.entity(*layer).with_children(|layer| {
        spawn_colour_window(
            layer,
            field.clone(),
            &swatch.label,
            painted.0,
            Vec2::new(left, TOP_MARGIN),
            *skin,
        );
    });
}

/// The picker, in one place: the frame, its bar, the preview and the four
/// channels.
fn spawn_colour_window(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    field: InspectorField,
    label: &str,
    colour: Color,
    at: Vec2,
    skin: UiSkin,
) {
    let mut frame = layer.spawn((
        Name::new("Colour Window"),
        EditorWindow,
        ColourWindow { field },
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x),
            top: px(at.y),
            width: px(WINDOW_W),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        panel(skin),
    ));
    let window = frame.id();
    frame.with_children(|frame| {
        frame
            .spawn((
                Name::new("Colour Window Bar"),
                WindowTitleBar { window },
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(8),
                    padding: UiRect::axes(px(10), px(6)),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    ..default()
                },
                BorderColor::all(theme::PHOSPHOR.with_alpha(0.16)),
                observe(on_window_drag),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Name::new("Colour Window Title"),
                    UiText,
                    // The ROW's name, not "colour": the window is four channel
                    // sliders and a swatch, so what it needs to say is which
                    // field it is on.
                    Text::new(label.to_uppercase()),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR),
                ));
                bar.spawn((
                    Name::new("Colour Window Close"),
                    WindowClose { window },
                    Node {
                        width: px(18),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Button,
                    Hovered::default(),
                    UiText,
                    Text::new("x"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                    observe(on_window_close),
                ));
            });
        frame
            .spawn((
                Name::new("Colour Window Body"),
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(6),
                    padding: UiRect::all(px(10)),
                    ..default()
                },
            ))
            .with_children(|body| {
                body.spawn((
                    Name::new("Colour Window Preview"),
                    ColourPreview { window },
                    Node {
                        width: percent(100),
                        height: px(30),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    BorderColor::all(theme::PHOSPHOR.with_alpha(0.4)),
                    BackgroundColor(colour),
                ));
                body.spawn((
                    Name::new("Colour Window Hex"),
                    ColourReadout { window },
                    UiText,
                    Text::new(colour_text(colour)),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ));
                for channel in [
                    ColourChannel::Red,
                    ColourChannel::Green,
                    ColourChannel::Blue,
                    ColourChannel::Alpha,
                ] {
                    spawn_channel(body, window, channel, colour, skin);
                }
            });
    });
}

/// One channel: its letter, its slider and its number.
fn spawn_channel(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    window: Entity,
    channel: ColourChannel,
    colour: Color,
    skin: UiSkin,
) {
    let value = channel.of(colour);
    body.spawn((
        Name::new(format!("Colour Window Row {}", channel.label())),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Name::new(format!("Colour Window Label {}", channel.label())),
            UiText,
            Text::new(channel.label()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
            Node {
                width: px(10),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        // The track is percent(100) wide, so it needs a cell of its own to
        // fill - the same shape the settings volume row uses.
        row.spawn(Node {
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|cell| {
            cell.spawn((
                Name::new(format!("Colour Window Slider {}", channel.label())),
                ColourSlider { window, channel },
                Slider {
                    track_click: TrackClick::Snap,
                    ..default()
                },
                SliderValue(value),
                SliderRange::new(0.0, 1.0),
                SliderStep(1.0 / 255.0),
                slider_track(value, skin),
            ));
        });
        row.spawn((
            Name::new(format!("Colour Window Value {}", channel.label())),
            ChannelReadout { window, channel },
            UiText,
            Text::new(channel_text(value)),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
            Node {
                width: px(26),
                flex_shrink: 0.0,
                ..default()
            },
        ));
    });
}

/// A channel as the 0-255 number a builder reads everywhere else a colour is
/// written down.
fn channel_text(value: f32) -> String {
    format!("{}", (value.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Drag a window by its bar, and keep enough of it on screen to drag back.
pub(crate) fn on_window_drag(
    drag: On<Pointer<Drag>>,
    bars: Query<&WindowTitleBar>,
    mut windows: Query<&mut Node, With<EditorWindow>>,
    screen: Option<Single<&Window>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Ok(bar) = bars.get(drag.entity) else {
        return;
    };
    let Ok(mut node) = windows.get_mut(bar.window) else {
        return;
    };
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        return;
    };
    node.left = px((left + drag.delta.x).clamp(
        KEEP_ON_SCREEN - WINDOW_W,
        (size.x - KEEP_ON_SCREEN).max(0.0),
    ));
    node.top = px((top + drag.delta.y).clamp(0.0, (size.y - KEEP_ON_SCREEN).max(0.0)));
}

/// Put a window away.
pub(crate) fn on_window_close(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<&WindowClose>,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    commands.entity(button.window).despawn();
}

/// Write a dragged channel back to the document.
///
/// The other three channels are read off the picker's OWN sliders rather than
/// off the config: this observer runs beside bevy's `slider_self_update`, and
/// which of the two lands first is not something either can see.
pub(crate) fn on_colour_slider(
    change: On<ValueChange<f32>>,
    sliders: Query<(&ColourSlider, &SliderValue)>,
    windows: Query<&ColourWindow>,
    mut targets: EditTargets,
    mut says: EditorSays,
) {
    let Ok((slider, _)) = sliders.get(change.source) else {
        return;
    };
    let Ok(window) = windows.get(slider.window) else {
        return;
    };
    let mut colour = Color::NONE;
    for (other, value) in &sliders {
        if other.window == slider.window {
            colour = other.channel.set(colour, value.0);
        }
    }
    let colour = slider.channel.set(colour, change.value);
    let written = targets.edit(&window.field, |root, path, optional| {
        write_field(root, path, optional, &colour_text(colour))
    });
    if let Err(reason) = written {
        says.refuse(reason);
    }
}

/// Show what the DOCUMENT says, and close a window whose row has gone.
///
/// The source is the Inspector's own swatch: it is painted from the config
/// every frame, so a colour changed by typing hex, by undo or by anything else
/// reaches the picker without a second read of the node. And a swatch that no
/// longer exists means the panel is on another node - the window belongs to
/// that row, so it goes with it.
pub(crate) fn sync_colour_windows(
    mut commands: Commands,
    windows: Query<(Entity, &ColourWindow)>,
    swatches: Query<(&InspectorField, &BackgroundColor), With<InspectorSwatch>>,
    sliders: Query<(Entity, &ColourSlider, &SliderValue)>,
    mut previews: Query<(&ColourPreview, &mut BackgroundColor), Without<InspectorSwatch>>,
    mut readouts: Query<(&ColourReadout, &mut Text), Without<ChannelReadout>>,
    mut channels: Query<(&ChannelReadout, &mut Text), Without<ColourReadout>>,
) {
    for (entity, window) in &windows {
        let Some(colour) = swatches
            .iter()
            .find(|(field, _)| **field == window.field)
            .map(|(_, painted)| painted.0)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        for (handle, slider, value) in &sliders {
            if slider.window != entity {
                continue;
            }
            let wanted = slider.channel.of(colour);
            // One 8-bit step of slack. The colour is written to the config as
            // hex and read back as hex, so an exact compare would fight the
            // drag that is happening: the value would be corrected to the
            // rounded one every frame the pointer moved.
            //
            // Written through commands because `SliderValue` is immutable -
            // bevy's own `slider_self_update` commits it the same way.
            if (value.0 - wanted).abs() > 0.5 / 255.0 {
                commands.entity(handle).insert(SliderValue(wanted));
            }
        }
        for (preview, mut painted) in &mut previews {
            if preview.window == entity && painted.0 != colour {
                *painted = colour.into();
            }
        }
        let hex = colour_text(colour);
        for (readout, mut text) in &mut readouts {
            if readout.window == entity && text.0 != hex {
                text.0.clone_from(&hex);
            }
        }
        for (readout, mut text) in &mut channels {
            if readout.window != entity {
                continue;
            }
            let wanted = channel_text(readout.channel.of(colour));
            if text.0 != wanted {
                text.0 = wanted;
            }
        }
    }
}

#[cfg(test)]
mod tests;
