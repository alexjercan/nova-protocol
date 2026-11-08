//! Floating windows: the layer they stand on, the bar you drag them by, and
//! the confirm window. The colour picker is one (`crate::ui::picker`).
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
    ui_widgets::{observe, Activate, Button},
};
use nova_gameplay::prelude::{GameStates, GuardedLeave, PauseStates, UnsavedWork};
use nova_ui::{
    prelude::{
        panel, scroll_bar, scroll_column, scroll_row, scroll_viewport, themed_button, UiText,
    },
    theme,
    theme::{density, UiColor},
    widget::{ButtonCueType, ThemedBorder, ThemedText},
};

use crate::{
    history::EditHistory,
    project::package::import_over,
    ui::{
        hub::{delete_local_mod, leave_to_hub},
        layer,
        menu::back_to_main_menu,
    },
    ExampleStates,
};

/// Window width. The inspector's own width, so a picker reads as the row it
/// came from rather than as a second panel with its own ideas - and follows it
/// when the panel is widened.
pub(crate) const WINDOW_W: f32 = crate::ui::inspector::PANEL_W;

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
    fn confirm(&self) -> &'static str {
        match self {
            Self::MainMenu => "Discard and leave",
            Self::Exit => "Discard and quit",
            Self::Hub => "Discard and close",
            Self::DeleteMod(_) => "Delete mod",
            Self::OverwriteImport(_) => "Replace it",
        }
    }

    /// The label on the safe answer.
    fn keep(&self) -> &'static str {
        match self {
            Self::DeleteMod(_) | Self::OverwriteImport(_) => "Keep it",
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
                    let label = verb.confirm();
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

#[cfg(test)]
mod tests;
