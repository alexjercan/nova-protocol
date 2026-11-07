//! Floating windows: the layer they stand on, the bar you drag them by, and
//! the colour picker that is the first thing to live in one.
//!
//! A window is for an edit that needs ROOM - more than a 240px inspector row
//! can hold - and it is still an edit of the node the Inspector is on. It is
//! not a second view of the document: a window BELONGS to the row it was
//! opened from, so it follows what that row says and goes away with it.
//!
//! There is one at a time, over the fixed panels of `ui::shell`: a window is a
//! detail of one row, so a second one would only hide the first.

use bevy::{
    ecs::{relationship::RelatedSpawnerCommands, system::SystemParam},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        observe, Activate, Button, Slider, SliderRange, SliderStep, SliderValue, TrackClick,
        ValueChange,
    },
};
use nova_gameplay::prelude::{GameStates, GuardedLeave, PauseStates, UnsavedWork};
use nova_ui::{
    prelude::{
        button, panel, scroll_bar, scroll_column, scroll_row, scroll_viewport, slider_track,
        themed_button, ButtonSpec, UiText,
    },
    theme,
    theme::{density, UiColor},
    widget::{ButtonCueType, ThemedBorder, ThemedRadius, ThemedText},
};

use crate::{
    config::EditorSays,
    history::{commit_edit, EditHistory},
    inspect::{colour_text, write_field},
    project::{
        package::import_over,
        verbs::{add_scenario, pressed},
    },
    template::ScenarioTemplate,
    ui::{
        hub::{delete_local_mod, leave_to_hub},
        inspector::{EditTargets, InspectorField, InspectorSwatch},
        layer,
        menu::back_to_main_menu,
    },
    ExampleStates,
};

/// Window width. The inspector's own width, so a picker reads as the row it
/// came from rather than as a second panel with its own ideas - and follows it
/// when the panel is widened.
pub(crate) const WINDOW_W: f32 = crate::ui::inspector::PANEL_W;

/// The gutter a fresh window keeps between itself and the Inspector.
const INSPECTOR_GUTTER: f32 = 12.0;
/// How far off the right edge a fresh window stands. Derived from the panel it
/// belongs to, so widening the Inspector moves the window instead of parking it
/// underneath.
const RIGHT_MARGIN: f32 = crate::ui::inspector::PANEL_W + INSPECTOR_GUTTER;
/// The gutter a fresh window keeps between itself and the rail.
const RAIL_GUTTER: f32 = 12.0;
/// Where a fresh window's top edge sits, under the top bar.
pub(crate) const TOP_MARGIN: f32 = 64.0;
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
    pub(crate) window: Entity,
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
        GlobalZIndex(layer::WINDOW_Z),
    )
}

/// A verb that needs the builder's yes before it runs.
///
/// On the menu ROW, not on the button that carries it out: the row asks, the
/// window's own button is what actually runs the verb.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub(crate) enum DestructiveVerb {
    /// File > New Scenario. Loses nothing; this window is where its template
    /// is picked.
    NewScenario,
    /// File > Back to Main Menu, which ends the session and the project.
    MainMenu,
    /// Quitting the app with unsaved work.
    Exit,
    /// File > New Mod and Open Mod: back to the hub, closing this mod.
    Hub,
    /// Deleting a local mod from the hub.
    DeleteMod(String),
    /// Importing a package over the local mod of the same id.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "only a native import overwrites a mod")
    )]
    OverwriteImport(String),
}

impl DestructiveVerb {
    /// The window's title.
    fn title(&self) -> &'static str {
        match self {
            Self::NewScenario => "NEW SCENARIO",
            Self::MainMenu => "BACK TO MAIN MENU",
            Self::Exit => "EXIT",
            Self::Hub => "CLOSE THIS MOD",
            Self::DeleteMod(_) => "DELETE MOD",
            Self::OverwriteImport(_) => "IMPORT OVER A MOD",
        }
    }

    /// What is about to be lost, said plainly.
    fn question(&self) -> &'static str {
        match self {
            Self::NewScenario => "Pick what the new scenario starts from. It joins this mod, and undo takes it back.",
            Self::MainMenu => "Leaving ends the session. Anything not saved goes with it.",
            Self::Exit => "Quitting ends the session. Anything not saved goes with it.",
            Self::Hub => "Going to the Mod Hub closes this mod. Anything not saved goes with it.",
            Self::DeleteMod(_) => "This deletes the mod and every file in it from this machine. Undo cannot bring it back.",
            Self::OverwriteImport(_) => "A mod with this id is already here. Importing replaces every file in it, and undo cannot bring it back.",
        }
    }

    /// The label on the button that goes through with it. It names the VERB
    /// rather than saying "OK", so a builder reading only the buttons still
    /// knows which one is the destructive one.
    ///
    /// `None` for New Scenario: there is no ONE thing it goes through with -
    /// the templates are the answers, and each is its own button.
    fn confirm(&self) -> Option<&'static str> {
        match self {
            Self::NewScenario => None,
            Self::MainMenu => Some("Discard and leave"),
            Self::Exit => Some("Discard and quit"),
            Self::Hub => Some("Discard and close"),
            Self::DeleteMod(_) => Some("Delete mod"),
            Self::OverwriteImport(_) => Some("Replace it"),
        }
    }

    /// The label on the safe answer.
    fn keep(&self) -> &'static str {
        match self {
            Self::DeleteMod(_) | Self::OverwriteImport(_) => "Keep it",
            Self::NewScenario => "Cancel",
            _ => "Keep editing",
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

/// Everything putting the question up needs.
#[derive(SystemParam)]
pub(crate) struct ConfirmAsk<'w, 's> {
    commands: Commands<'w, 's>,
    layer: Option<Single<'w, 's, Entity, With<EditorWindowLayer>>>,
    open: Query<'w, 's, (), With<ConfirmWindow>>,
    screen: Option<Single<'w, 's, &'static Window>>,
}

impl ConfirmAsk<'_, '_> {
    /// Whether the window can go up: the editor chrome stands.
    pub(crate) fn ready(&self) -> bool {
        self.layer.is_some()
    }

    /// Put the question up, unless one already stands.
    pub(crate) fn ask(&mut self, verb: DestructiveVerb) {
        let Some(layer) = self.layer.as_deref().copied() else {
            return;
        };
        if !self.open.is_empty() {
            return;
        }
        let size = self
            .screen
            .as_deref()
            .map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
        let at = Vec2::new(((size.x - CONFIRM_W) * 0.5).max(8.0), TOP_MARGIN);
        self.commands
            .entity(layer)
            .with_children(|layer| spawn_confirm_window(layer, verb, at));
    }
}

/// Put the question up instead of doing it.
///
/// One observer for every verb, keyed on the row's own [`DestructiveVerb`].
/// The button inside the window carries the real verb's observer, so there is
/// no second copy of what any of them do.
///
/// Leaving a mod with nothing unsaved loses nothing, so it asks nothing.
/// New Scenario always asks: the window is also where its template is picked.
pub(crate) fn on_destructive_item(
    activate: On<Activate>,
    rows: Query<&DestructiveVerb>,
    history: Res<EditHistory>,
    mut game_state: ResMut<NextState<GameStates>>,
    mut confirm: ConfirmAsk,
) {
    let Ok(verb) = rows.get(activate.entity) else {
        return;
    };
    if !history.dirty() {
        match verb {
            DestructiveVerb::MainMenu => {
                game_state.set(GameStates::MainMenu);
                return;
            }
            DestructiveVerb::Hub => {
                confirm.commands.queue(leave_to_hub);
                return;
            }
            _ => {}
        }
    }
    confirm.ask(verb.clone());
}

/// A leave that `nova_menu` or `nova_scenario` stopped for unsaved work: go
/// back to the editor and ask there, because the question is the editor's.
///
/// The leave waits in a `Local` across the frames it takes the chrome to stand
/// up after a stop from inside the flown scenario.
pub(crate) fn ask_about_stopped_leave(
    mut unsaved: ResMut<UnsavedWork>,
    mut waiting: Local<Option<GuardedLeave>>,
    state: Res<State<ExampleStates>>,
    mut next_state: ResMut<NextState<ExampleStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
    mut confirm: ConfirmAsk,
) {
    if let Some(leave) = unsaved.take_asked() {
        *waiting = Some(leave);
        pause.set(PauseStates::Unpaused);
        if *state.get() == ExampleStates::Scenario {
            next_state.set(ExampleStates::Editor);
        }
    }
    if !confirm.ready() {
        return;
    }
    if let Some(leave) = waiting.take() {
        confirm.ask(match leave {
            GuardedLeave::MainMenu => DestructiveVerb::MainMenu,
            GuardedLeave::Exit => DestructiveVerb::Exit,
        });
    }
}

/// The question, its two answers, and the bar you can drag it by.
fn spawn_confirm_window(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    verb: DestructiveVerb,
    at: Vec2,
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
        panel(),
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
                    column_gap: px(density::GAP),
                    padding: UiRect::axes(px(density::PANEL_PAD + 1.0), px(2)),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    ..default()
                },
                BorderColor::all(Color::NONE),
                ThemedBorder::alpha(UiColor::Primary, 0.16),
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
                    TextColor(Color::NONE),
                    ThemedText::new(UiColor::Accent),
                ));
            });
        frame
            .spawn((
                Name::new("Confirm Window Body"),
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(density::GAP),
                    padding: UiRect::all(px(density::PANEL_PAD)),
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
                    TextColor(Color::NONE),
                    ThemedText::new(UiColor::Primary),
                ));
                // The templates, where the verb has no single answer: each
                // row IS an answer, and pressing one both picks the world and
                // goes through with the discard.
                if verb == DestructiveVerb::NewScenario {
                    body.spawn((
                        Name::new("Template List"),
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: px(2),
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        for template in ScenarioTemplate::ALL {
                            list.spawn((
                                Name::new(format!("Template {}", template.label())),
                                template,
                                ConfirmAnswer,
                                button(ButtonSpec::new(template.label()).dense().block()),
                                ButtonCueType::Owned,
                                observe(on_template_row),
                            ));
                            list.spawn((
                                Name::new(format!("Template Hint {}", template.label())),
                                UiText,
                                Text::new(template.hint()),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::NONE),
                                ThemedText::new(UiColor::Label),
                                Node {
                                    margin: UiRect::bottom(px(6)),
                                    ..default()
                                },
                            ));
                        }
                    });
                }
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
                                themed_button(verb.keep()),
                                ButtonCueType::Back,
                            ));
                        });
                    let Some(label) = verb.confirm() else {
                        return;
                    };
                    answers
                        .spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        })
                        .with_children(|slot| {
                            let mut go = slot.spawn((
                                Name::new("Confirm Discard Button"),
                                ConfirmAnswer,
                                themed_button(label),
                            ));
                            match verb {
                                DestructiveVerb::MainMenu => go.observe(back_to_main_menu),
                                DestructiveVerb::Exit => go.observe(quit_app),
                                DestructiveVerb::Hub => {
                                    go.observe(|_: On<Activate>, mut commands: Commands| {
                                        commands.queue(leave_to_hub);
                                    })
                                }
                                DestructiveVerb::DeleteMod(id) => {
                                    go.observe(move |_: On<Activate>, mut commands: Commands| {
                                        let id = id.clone();
                                        commands.queue(move |world: &mut World| {
                                            delete_local_mod(world, &id);
                                        });
                                    })
                                }
                                DestructiveVerb::OverwriteImport(path) => {
                                    go.observe(move |_: On<Activate>, mut commands: Commands| {
                                        let path = path.clone();
                                        commands.queue(move |world: &mut World| {
                                            import_over(world, &path);
                                        });
                                    })
                                }
                                // Answered by the template rows above.
                                DestructiveVerb::NewScenario => unreachable!(),
                            };
                        });
                });
            });
    });
}

/// Discard and quit.
fn quit_app(_activate: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

/// A template row: add the scenario it starts.
fn on_template_row(
    activate: On<Activate>,
    mut commands: Commands,
    templates: Query<&ScenarioTemplate>,
) {
    if let Ok(template) = templates.get(activate.entity).copied() {
        commands.queue(move |world: &mut World| {
            pressed(world, |world| add_scenario(world, template));
        });
    }
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
    let left = fresh_window_left(size.x);
    commands.entity(*layer).with_children(|layer| {
        spawn_colour_window(
            layer,
            field.clone(),
            &swatch.label,
            painted.0,
            Vec2::new(left, TOP_MARGIN),
        );
    });
}

/// Where a fresh window's left edge goes on a window `width` logical px wide.
///
/// Right-anchored, then bounded off the rail. At 760 logical px the band
/// between the rail and the Inspector is 250 wide and the window is 300, so
/// there is no place that clears both: it clears the RAIL and lets its right
/// edge run under the Inspector - the panel the picker came from and
/// duplicates - because the rail is the scene tree, which a colour has nothing
/// to do with.
fn fresh_window_left(width: f32) -> f32 {
    (width - RIGHT_MARGIN - WINDOW_W).max(crate::ui::RAIL_W + RAIL_GUTTER)
}

/// The frame every floating window shares: the panel, the bar it is dragged
/// by, the title, the close, and a body the caller fills.
pub(crate) fn window_frame(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    name: &str,
    title: &str,
    at: Vec2,
    held: impl Bundle,
    body: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>, Entity),
) -> Entity {
    let mut frame = layer.spawn((
        Name::new(name.to_string()),
        EditorWindow,
        held,
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x),
            top: px(at.y),
            width: px(WINDOW_W),
            // Bounded, and its body scrolls: a choice list is twenty-six
            // options long, and a frame that grew to hold them all ran off the
            // bottom of the screen with the last of them on it.
            max_height: percent(80),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        panel(),
    ));
    let window = frame.id();
    let bar_name = format!("{name} Bar");
    let close_name = format!("{name} Close");
    let body_name = format!("{name} Body");
    frame.with_children(|frame| {
        frame
            .spawn((
                Name::new(bar_name),
                WindowTitleBar { window },
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(density::GAP),
                    padding: UiRect::axes(px(density::PANEL_PAD + 1.0), px(2)),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    ..default()
                },
                BorderColor::all(Color::NONE),
                ThemedBorder::alpha(UiColor::Primary, 0.16),
                observe(on_window_drag),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Name::new(format!("{name} Title")),
                    // The ROW's name: a window is a control for one field, so
                    // what it has to say is which field it is on.
                    UiText,
                    Text::new(title.to_uppercase()),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    ThemedText::new(UiColor::Primary),
                ));
                bar.spawn((
                    Name::new(close_name),
                    WindowClose { window },
                    Node {
                        width: px(18),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Button,
                    ButtonCueType::Back,
                    Hovered::default(),
                    UiText,
                    Text::new("x"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    ThemedText::new(UiColor::Label),
                    observe(on_window_close),
                ));
            });
        frame
            .spawn((Name::new(format!("{name} Scroll")), scroll_row()))
            .with_children(|scroll| {
                scroll
                    .spawn((
                        Name::new(body_name),
                        Node {
                            align_items: AlignItems::Stretch,
                            row_gap: px(density::GAP),
                            padding: UiRect::all(px(density::PANEL_PAD)),
                            ..scroll_column()
                        },
                        scroll_viewport(),
                    ))
                    .with_children(|filled| body(filled, window));
                scroll.spawn((Name::new(format!("{name} Scrollbar")), scroll_bar()));
            });
    });
    window
}

/// The picker, in one place: the frame, its bar, the preview and the four
/// channels.
fn spawn_colour_window(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    field: InspectorField,
    label: &str,
    colour: Color,
    at: Vec2,
) {
    window_frame(
        layer,
        "Colour Window",
        label,
        at,
        ColourWindow { field },
        |body, window| {
            body.spawn((
                Name::new("Colour Window Preview"),
                ColourPreview { window },
                Node {
                    width: percent(100),
                    height: px(30),
                    border: UiRect::all(px(theme::BORDER_W)),
                    ..default()
                },
                ThemedRadius::control(),
                BorderColor::all(Color::NONE),
                ThemedBorder::alpha(UiColor::Primary, 0.4),
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
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
            ));
            for channel in [
                ColourChannel::Red,
                ColourChannel::Green,
                ColourChannel::Blue,
                ColourChannel::Alpha,
            ] {
                spawn_channel(body, window, channel, colour);
            }
        },
    );
}

/// One channel: its letter, its slider and its number.
fn spawn_channel(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    window: Entity,
    channel: ColourChannel,
    colour: Color,
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
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Label),
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
                slider_track(value),
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
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Primary),
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
    mut commands: Commands,
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
    match written {
        // A drag writes every frame and is one edit. Its end is `is_final`.
        Ok(()) if change.is_final => commit_edit(&mut commands, window.field.edit_label()),
        Ok(()) => {}
        Err(reason) => says.flag(reason),
    }
}

/// A click on a channel's track moves it without any drag, so `is_final` never
/// comes. The release is what ends that gesture.
pub(crate) fn on_colour_release(
    release: On<Pointer<Release>>,
    mut commands: Commands,
    sliders: Query<&ColourSlider>,
    windows: Query<&ColourWindow>,
    parents: Query<&ChildOf>,
) {
    let held = std::iter::once(release.entity)
        .chain(parents.iter_ancestors(release.entity))
        .find_map(|entity| sliders.get(entity).ok());
    if let Some(window) = held.and_then(|slider| windows.get(slider.window).ok()) {
        commit_edit(&mut commands, window.field.edit_label());
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
