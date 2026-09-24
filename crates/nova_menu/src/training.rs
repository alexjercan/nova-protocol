//! The Training handbook: a main-menu modal of short, visual lessons, reached
//! from the `Lessons` row of the menu card, and the first-launch prompt in the
//! bottom-left corner.
//!
//! The screen is the menu's own list-beside-details language ([`nova_ui::screen`]),
//! NOT the NOVA OS terminal: there is no CRT casing, no prompt and no text
//! input anywhere in here. The lesson list is on the left under its category
//! headers; on the right one tip fills the pane - a 16:9 demonstration across
//! the top and a text box holding everything else under it.
//!
//! ONE TIP, ONE SCREEN. There is no page cursor and no Prev/Next: anything that
//! would have been a second page is its own lesson with its own id. That is
//! what lets the list be the table of contents and lets Viewed mean the player
//! saw a whole thing.
//!
//! TWO SURFACES, TWO JOBS. The `Lessons` row in the menu card is the permanent
//! way in and is always there, and the handbook's first row always launches
//! Basic Training. The bottom-left corner is a NOTICE CENTRE, not a
//! training widget: it holds the one-off OFFER to a player who has just
//! installed the game, and under it a field note - one fact read off the
//! catalog, with the way into the lesson that owns it.
//!
//! The prompt is remembered as a SETTING ([`TrainingPromptSetting`]), not as
//! progress. Starting Basic Training or answering `Not now` writes `Hidden`,
//! and Settings > Interface is where it comes back. The field note is switched
//! the same way ([`FieldNoteSetting`]): `Don't show again` on the card writes
//! `Hidden`, and the same Settings tab brings it back.
//!
//! THE LESSONS ARE CONTENT. `TrainingCatalog` is merged by `register_bundles`
//! from the base bundle's `Lesson` items plus every enabled mod's; this module
//! draws whatever it is handed and names no lesson id anywhere.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_hud::prelude::{KeyGlyphs, NovaHudAssets};
use nova_input::prelude::InputBindings;
use nova_training::prelude::*;
use nova_ui::{
    screen::{
        details_pane, footer_back_slot, list_detail_screen, list_pane, overlay_root, scroll_bar,
        scroll_column, scroll_row, scroll_viewport,
    },
    theme,
    theme::UiColor,
    widget::{
        badge, list_row, panel, panel_header, BadgeKind, ButtonSpec, Selected, ThemedBorder,
        ThemedFill, ThemedRadius, ThemedText, UiText,
    },
};

use crate::{
    mods::ModsPanel,
    scenarios::{NewGameScenario, ScenariosPanel},
    settings::{spawn_binding_chips, FieldNoteSetting, SettingsPanel, TrainingPromptSetting},
    widgets::{back_button, button, button_variant},
};

/// Marker for the Training modal root, toggled by the Training button.
#[derive(Component)]
pub(crate) struct TrainingPanel;

/// The scrollable column holding the category headers and lesson rows.
#[derive(Component)]
pub(crate) struct TrainingList;

/// The lesson details side panel: title, pages, bindings and actions.
#[derive(Component)]
pub(crate) struct TrainingDetailsPanel;

/// The one-line progress summary above the two panes.
#[derive(Component)]
pub(crate) struct TrainingProgressLabel;

/// One clickable lesson row.
#[derive(Component)]
pub(crate) struct LessonRow {
    pub(crate) id: LessonId,
}

/// A lesson row's state badge, carrying the state it was drawn for so
/// [`sync_lesson_rows`] can tell a badge that is still right from one that is
/// stale without re-reading its text.
#[derive(Component)]
pub(crate) struct LessonStatusBadge(pub(crate) LessonStatus);

/// A launch into a scenario from the handbook: the details-pane Practice
/// button with the practice range it hands off to, and the Basic Training row
/// at the head of the list.
///
/// The Practice LABEL is the screen's, not the lesson's: every practice button
/// says the same word, so the player learns one control rather than reading a
/// different invitation on every tip.
#[derive(Component)]
pub(crate) struct LessonPractice {
    pub(crate) scenario: String,
}

/// What every Practice button says.
const PRACTICE_LABEL: &str = "Practice";

/// The bottom-left notice corner. Hidden while any menu modal is open, so
/// nothing reaches past the modal. What it HOLDS decides its own visibility:
/// the corner is a notice centre, not a single notice.
#[derive(Component)]
pub(crate) struct MenuAside;

/// The first-launch offer to fly Basic Training, one notice in the corner.
/// Hidden for good once the player has answered it.
#[derive(Component)]
pub(crate) struct TrainingPromptCard;

/// The field-note notice: one fact from the handbook, and the way into the
/// lesson it came from.
#[derive(Component)]
pub(crate) struct MenuFieldNoteCard;

/// The note card's `Open lesson` action, and the lesson it opens.
#[derive(Component)]
pub(crate) struct MenuFieldNoteLesson {
    pub(crate) id: LessonId,
}

/// The selected lesson's demonstration, held while it finishes loading.
///
/// The same two-step the scenario picker's thumbnail takes, and for the same
/// reason: an image can only be validated once it is IN memory (is it a plain
/// 2D texture the UI pipeline can bind?), so a still-loading handle is parked
/// here and [`poll_lesson_media`] re-arms the details refresh when it lands.
/// Without it a lesson opened before its art loaded would show the fallback
/// until the player clicked something else.
#[derive(Resource, Default)]
pub(crate) struct PendingLessonMedia(pub(crate) Option<Handle<Image>>);

/// A running sprite-sheet demonstration: which cell to draw next, and when.
///
/// The sheet is cut by the AUTHORED grid, not by anything in the file - an
/// image says nothing about where its cells are - so the frame count lives here and
/// the layout was built from the lesson's own `columns`/`rows`.
#[derive(Component)]
pub(crate) struct LessonLoop {
    frames: u32,
    timer: Timer,
}

/// The lesson the details pane renders. `None` until the list populates -
/// `refresh_training_list` default-selects the first row and repairs a stale
/// pick.
#[derive(Resource, Default)]
pub(crate) struct SelectedLessonId(pub(crate) Option<LessonId>);

/// How wide the notice corner is, in logical pixels.
///
/// Wide enough to read across the room, because this is a first impression a
/// player meets once. At the supported narrow window (640 wide) the
/// `max_width: 40%` beside it wins instead: 40 percent of 640 is 256, and the
/// menu card is 280 wide with a 40-pixel margin, so the two never touch.
const ASIDE_W: f32 = 520.0;

/// The modal's share of the window, matching the Mods and Scenarios screens.
const PANEL_PCT: f32 = 85.0;

/// Spawn the Training modal, hidden until the Training button toggles it.
///
/// Both panes spawn EMPTY: writing [`SelectedLessonId`] marks it changed, which
/// re-arms the refreshers to fill them on the first Update after entry - one
/// population path for entry, selection and a live catalog change alike, the
/// same shape the mods and scenarios screens use.
pub(crate) fn spawn_training_panel(commands: &mut Commands) {
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Training Panel Root"),
            TrainingPanel,
            overlay_root(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Training Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: percent(PANEL_PCT),
                        height: percent(PANEL_PCT),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::panel(),
                    panel(),
                ))
                .with_children(|parent| {
                    // Title, then subtitle, exactly as the Mods and Scenarios
                    // screens open - a modal that announced itself differently
                    // from its siblings would read as a different product.
                    // The progress summary rides the title row rather than
                    // taking a third line of its own: it is a readout, not a
                    // heading, and the header is competing with the lesson list
                    // for the top of a 600px window.
                    parent.spawn((
                        Name::new("Training Head"),
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: px(16),
                            ..default()
                        },
                        children![
                            (
                                Name::new("Training Title"),
                                UiText,
                                Text::new("Training"),
                                TextFont {
                                    font_size: FontSize::Px(24.0),
                                    ..default()
                                },
                                TextColor(Color::NONE),
                                ThemedText::new(UiColor::Body),
                            ),
                            (
                                Name::new("Training Progress"),
                                TrainingProgressLabel,
                                UiText,
                                Text::new(String::new()),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::NONE),
                                ThemedText::new(UiColor::Primary),
                            ),
                        ],
                    ));
                    parent.spawn((
                        Name::new("Training Subtitle"),
                        UiText,
                        Text::new(
                            "Short lessons on how to fly, build and fight. The \
                             complete manual is on the wiki.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));

                    parent.spawn((
                        Name::new("Training Content"),
                        list_detail_screen(
                            (
                                Name::new("Training Left Pane"),
                                list_pane(),
                                children![(
                                    Name::new("Training List Row"),
                                    scroll_row(),
                                    children![
                                        (
                                            Name::new("Training List"),
                                            TrainingList,
                                            scroll_column(),
                                            scroll_viewport(),
                                        ),
                                        (Name::new("Training List Scroll Bar"), scroll_bar(),),
                                    ],
                                )],
                            ),
                            (
                                Name::new("Training Details Panel"),
                                TrainingDetailsPanel,
                                details_pane(),
                            ),
                        ),
                    ));

                    parent.spawn((
                        Name::new("Training Footer"),
                        footer_back_slot((
                            Name::new("Training Back Button"),
                            back_button("Back"),
                            observe(on_training_back),
                        )),
                    ));
                });
        });
}

/// Spawn the bottom-left notice corner and the notices in it: the first-launch
/// offer to fly Basic Training, and one field note from the handbook.
///
/// BOTTOM-LEFT because the menu card is bottom-right and the backdrop owns the
/// middle. The corner is built once per menu entry and never reconciled - what
/// it holds does not change while the player looks at it, which is exactly the
/// "do not rotate it while visible" rule the note needs. Answering the prompt
/// writes [`TrainingPromptSetting::Hidden`] and `sync_menu_aside` takes THAT
/// notice down on the next frame; the note stays, because it was never an
/// offer to answer.
///
/// The note is the LAST child, so the corner grows upward from a fixed bottom
/// edge and the note sits in the same place whether the prompt is above it or
/// not.
pub(crate) fn spawn_menu_aside(commands: &mut Commands, note: Option<&FieldNote>) {
    let aside = commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Menu Aside"),
            MenuAside,
            Node {
                position_type: PositionType::Absolute,
                left: px(40),
                bottom: px(40),
                width: px(ASIDE_W),
                // 40 percent, not a rounder number: the menu card is 280px wide at
                // right:40, so at the 640px floor the corner has
                // 640 - 280 - 40 - 40 = 280px before the two cards touch. 40
                // percent (256px there) leaves a gap, and on a desk monitor the
                // fixed width wins long before the percentage does.
                max_width: percent(40),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            children![(
                Name::new("Training Prompt"),
                TrainingPromptCard,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(14)),
                    row_gap: px(4),
                    border: UiRect::all(px(theme::BORDER_W)),
                    ..default()
                },
                ThemedRadius::control(),
                panel(),
                children![
                    (
                        Name::new("Training Prompt Header"),
                        panel_header("New pilot")
                    ),
                    (
                        Name::new("Training Prompt Blurb"),
                        UiText,
                        Text::new(
                            "Never flown one of these? Basic Training covers the \
                         controls in a few minutes.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Body),
                    ),
                    (
                        Name::new("Training Prompt Actions"),
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                            row_gap: px(6),
                            margin: UiRect::top(px(8)),
                            ..default()
                        },
                        children![
                            (
                                Name::new("Training Prompt Start"),
                                button_variant(
                                    "Start Basic Training",
                                    nova_ui::widget::ButtonVariant::Primary,
                                    None,
                                ),
                                observe(on_first_pilot_start),
                            ),
                            (
                                Name::new("Training Prompt Lessons"),
                                button("Open lessons"),
                                observe(on_open_handbook),
                            ),
                            (
                                Name::new("Training Prompt Dismiss"),
                                nova_ui::widget::button(ButtonSpec::new("Not now").ghost().fit()),
                                crate::widgets::MenuSfxBack,
                                crate::widgets::MenuSfxButton,
                                observe(on_first_pilot_dismiss),
                            ),
                        ],
                    ),
                ],
            )],
        ))
        .id();
    if let Some(note) = note {
        spawn_field_note_card(commands, aside, note);
    }
}

/// The field-note notice: the same two lines the loading screens draw, plus the
/// way into the lesson that owns the claim.
///
/// The claim itself is NOT written here - it is read off the catalog, so the
/// card and the lesson cannot drift apart. The `Open lesson` action only
/// appears for a note that came from a lesson; a compiled boot note has no page
/// to open. `Don't show again` is on EVERY note, because a player who does not
/// want facts in the corner does not want them either way.
fn spawn_field_note_card(commands: &mut Commands, aside: Entity, note: &FieldNote) {
    commands.entity(aside).with_children(|corner| {
        corner
            .spawn((
                Name::new("Menu Field Note"),
                MenuFieldNoteCard,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(14)),
                    row_gap: px(4),
                    border: UiRect::all(px(theme::BORDER_W)),
                    ..default()
                },
                ThemedRadius::control(),
                panel(),
            ))
            .with_children(|card| {
                card.spawn((
                    Name::new("Menu Field Note Header"),
                    panel_header("Field note"),
                ));
                for line in &note.lines {
                    card.spawn((
                        Name::new("Menu Field Note Line"),
                        UiText,
                        Text::new(line.clone()),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Body),
                    ));
                }
                card.spawn((
                    Name::new("Menu Field Note Actions"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        row_gap: px(6),
                        margin: UiRect::top(px(8)),
                        ..default()
                    },
                ))
                .with_children(|actions| {
                    if let Some(lesson) = &note.lesson {
                        actions.spawn((
                            Name::new("Menu Field Note Lesson Button"),
                            MenuFieldNoteLesson { id: lesson.clone() },
                            nova_ui::widget::button(ButtonSpec::new("Open lesson").ghost().fit()),
                            crate::widgets::MenuSfxButton,
                            observe(on_open_note_lesson),
                        ));
                    }
                    actions.spawn((
                        Name::new("Menu Field Note Dismiss"),
                        nova_ui::widget::button(ButtonSpec::new("Don't show again").ghost().fit()),
                        crate::widgets::MenuSfxBack,
                        crate::widgets::MenuSfxButton,
                        observe(on_dismiss_field_notes),
                    ));
                });
            });
    });
}

// -- Observers --

/// Open or close the handbook from the menu card's `Lessons` row.
///
/// It does NOT touch the selection: the list default-selects, and a default
/// selection deliberately marks nothing Viewed. `Open lessons` on the prompt
/// is the one that puts the player on Start Here on purpose.
pub(crate) fn on_training(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<TrainingPanel>>,
) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

pub(crate) fn on_training_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<TrainingPanel>>,
) {
    **panel = Visibility::Hidden;
}

/// Select a lesson: the ONE path into [`SelectedLessonId`] that marks a lesson
/// Viewed.
///
/// A default or repaired selection deliberately does NOT come through here.
/// Opening the handbook puts a lesson on screen without the player having
/// chosen it, and a record that called that reading would have every player
/// arrive with the first lesson already ticked off.
fn select_lesson(id: &str, selected: &mut SelectedLessonId, progress: &mut TrainingProgress) {
    selected.0 = Some(id.to_string());
    progress.mark_viewed(id);
}

pub(crate) fn on_lesson_row_select(
    activate: On<Activate>,
    rows: Query<&LessonRow>,
    mut selected: ResMut<SelectedLessonId>,
    mut progress: ResMut<TrainingProgress>,
) {
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    select_lesson(&row.id, &mut selected, &mut progress);
}

/// Launch a [`LessonPractice`] scenario through the SAME New Game hand-off the
/// Scenarios picker uses. There is no second launch path.
pub(crate) fn on_lesson_practice(
    activate: On<Activate>,
    practices: Query<&LessonPractice>,
    mut pick: ResMut<NewGameScenario>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    let Ok(practice) = practices.get(activate.entity) else {
        return;
    };
    pick.0 = Some(practice.scenario.clone());
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
}

/// `Start Basic Training` plays Basic Training through the same New Game
/// hand-off the Scenarios picker uses. New Game itself opens the world setup
/// modal, so the offer names its scenario instead of taking the declared start.
///
/// Taking the offer answers it: the prompt does not come back on the next
/// launch, and Settings > Interface is where a player who wants it back goes.
pub(crate) fn on_first_pilot_start(
    _activate: On<Activate>,
    mut pick: ResMut<NewGameScenario>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
    mut prompt: ResMut<TrainingPromptSetting>,
) {
    *prompt = TrainingPromptSetting::Hidden;
    pick.0 = Some(TUTORIAL_SCENARIO_ID.to_string());
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
}

/// The prompt's lessons action opens the handbook on the first lesson - Start
/// Here. Reading is not answering the offer, so this leaves the prompt alone.
pub(crate) fn on_open_handbook(
    _activate: On<Activate>,
    catalog: Res<TrainingCatalog>,
    mut panel: Single<&mut Visibility, With<TrainingPanel>>,
    mut selected: ResMut<SelectedLessonId>,
    mut progress: ResMut<TrainingProgress>,
) {
    if let Some(id) = catalog.first_id().cloned() {
        select_lesson(&id, &mut selected, &mut progress);
    }
    **panel = Visibility::Visible;
}

/// The note card's `Open lesson` opens the handbook AT the lesson the fact came
/// from, which is the whole reason the card carries one. Choosing it is a
/// choice, so unlike a default selection it marks the lesson Viewed.
pub(crate) fn on_open_note_lesson(
    activate: On<Activate>,
    notes: Query<&MenuFieldNoteLesson>,
    mut panel: Single<&mut Visibility, With<TrainingPanel>>,
    mut selected: ResMut<SelectedLessonId>,
    mut progress: ResMut<TrainingProgress>,
) {
    let Ok(note) = notes.get(activate.entity) else {
        return;
    };
    select_lesson(&note.id, &mut selected, &mut progress);
    **panel = Visibility::Visible;
}

/// `Don't show again` on the note card switches the corner's field notes off.
///
/// The same shape as `Not now` on the offer: a card carries the way to turn
/// itself off, and Settings > Interface carries the way back. It is the SETTING
/// that is written, not the card - `sync_menu_aside` takes the notice down on
/// the next frame, and the next launch reads the saved answer.
pub(crate) fn on_dismiss_field_notes(_activate: On<Activate>, mut notes: ResMut<FieldNoteSetting>) {
    *notes = FieldNoteSetting::Hidden;
}

/// `Not now` is the other way to answer the offer. It is a SETTING, so the
/// answer survives a restart without the handbook owning a save file of its
/// own.
pub(crate) fn on_first_pilot_dismiss(
    _activate: On<Activate>,
    mut prompt: ResMut<TrainingPromptSetting>,
) {
    *prompt = TrainingPromptSetting::Hidden;
}

// -- Refreshers --

pub(crate) fn training_list_dirty(
    catalog: Res<TrainingCatalog>,
    spawned: Query<(), Added<TrainingList>>,
) -> bool {
    // The just-spawned pane is the load-bearing trigger, not a convenience.
    // The catalog is inserted when the plugin is built, which is BEFORE the
    // `Update` schedule first initializes, so its change tick is already old
    // the first time this condition is asked. The menu, by contrast, is
    // rebuilt on every entry to it: an empty pane is always newly spawned, and
    // that is what this reads.
    //
    // Neither the SELECTION nor the PROGRESS record is a signal here. One click
    // moves both (`select_lesson` marks the lesson Viewed), and rebuilding on
    // either despawned and respawned all sixty-odd rows to move one highlight
    // one place - the flash the handbook twitched with, and the scroll position
    // with it. `sync_lesson_rows` moves the highlight and rewrites the one
    // badge that changed instead.
    !spawned.is_empty() || catalog.is_changed()
}

/// The one-line count above the two panes. Written by both the rebuild and the
/// per-row sync, because the record moves on a click the list no longer
/// rebuilds for - and a stale "0 opened" over a row that reads READ is the
/// screen disagreeing with itself.
fn write_progress_label(
    catalog: &TrainingCatalog,
    progress: &TrainingProgress,
    labels: &mut Query<&mut Text, With<TrainingProgressLabel>>,
) {
    let (viewed, completed, total) = progress.counts(catalog);
    for mut label in labels {
        label.0 = format!("{completed} of {total} completed - {viewed} opened");
    }
}

/// Move the row highlight and the row state badges without rebuilding the list.
///
/// The counterpart to the two signals [`training_list_dirty`] no longer reads.
/// Only rows whose highlight or state actually moved are touched, so the list
/// under the pointer holds still.
pub(crate) fn sync_lesson_rows(
    mut commands: Commands,
    catalog: Res<TrainingCatalog>,
    selected: Res<SelectedLessonId>,
    progress: Res<TrainingProgress>,
    rows: Query<(Entity, &LessonRow, Has<Selected>, Option<&Children>)>,
    badges: Query<(Entity, &LessonStatusBadge)>,
    mut labels: Query<&mut Text, With<TrainingProgressLabel>>,
) {
    if !selected.is_changed() && !progress.is_changed() {
        return;
    }
    write_progress_label(&catalog, &progress, &mut labels);
    for (entity, row, marked, children) in &rows {
        let highlighted = selected.0.as_deref() == Some(row.id.as_str());
        if highlighted != marked {
            // `try_*`: a row can be despawned the same frame this deferred
            // command is queued (the menu tearing down under a state change).
            let mut ent = commands.entity(entity);
            if highlighted {
                ent.try_insert(Selected);
            } else {
                ent.try_remove::<Selected>();
            }
        }
        let status = progress.status(&row.id);
        // An untouched lesson draws no badge at all, so what SHOULD be on the
        // row is an `Option` too, and "nothing, correctly" has to compare equal
        // to "nothing" rather than count as a change every frame.
        let wanted = status_badge(status).map(|_| status);
        let drawn = children
            .into_iter()
            .flatten()
            .find_map(|child| badges.get(*child).ok());
        if drawn.map(|(_, badge)| badge.0) == wanted {
            continue;
        }
        if let Some((badge_entity, _)) = drawn {
            commands.entity(badge_entity).try_despawn();
        }
        if let Some((kind, text)) = status_badge(status) {
            let name = format!("Lesson Status: {}", row.id);
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Name::new(name),
                    LessonStatusBadge(status),
                    badge(kind, text),
                ));
            });
        }
    }
}

pub(crate) fn training_details_dirty(
    catalog: Res<TrainingCatalog>,
    progress: Res<TrainingProgress>,
    selected: Res<SelectedLessonId>,
    bindings: Option<Res<InputBindings>>,
    spawned: Query<(), Added<TrainingDetailsPanel>>,
) -> bool {
    // Same just-spawned rule as the list: see `training_list_dirty`.
    !spawned.is_empty()
        || catalog.is_changed()
        || progress.is_changed()
        || selected.is_changed()
        // A rebind taken in Settings must reach a chip the handbook is already
        // drawing: the lesson names the ACTION, and the chip is the answer.
        || bindings.is_some_and(|bindings| bindings.is_changed())
}

/// Rebuild the lesson list: the Basic Training row, then a header per non-empty
/// category and its lessons as rows carrying their state. A default/repaired selection keeps the details
/// pane fed without claiming the player read anything.
pub(crate) fn refresh_training_list(
    mut commands: Commands,
    catalog: Res<TrainingCatalog>,
    progress: Res<TrainingProgress>,
    mut selected: ResMut<SelectedLessonId>,
    lists: Query<Entity, With<TrainingList>>,
    mut labels: Query<&mut Text, With<TrainingProgressLabel>>,
) {
    let Ok(list) = lists.single() else {
        return;
    };
    commands.entity(list).despawn_related::<Children>();

    if !selected
        .0
        .as_deref()
        .is_some_and(|id| catalog.get(id).is_some())
    {
        // Bypasses `select_lesson` on purpose: see its doc comment.
        selected.0 = catalog.first_id().cloned();
    }

    write_progress_label(&catalog, &progress, &mut labels);

    commands.entity(list).with_children(|list| {
        // Basic Training heads the list, above every category, because New
        // Game opens the open world: this row is the handbook's own door into
        // the first-flight course. It launches, not selects - it is the
        // practice hand-off with no lesson in front of it, so it names the
        // scenario and never touches the selection or the record.
        list.spawn((
            Name::new("Training Basic Training"),
            LessonPractice {
                scenario: TUTORIAL_SCENARIO_ID.to_string(),
            },
            list_row(),
            Button,
            Hovered::default(),
            observe(on_lesson_practice),
            children![(
                Name::new("Training Basic Training Title"),
                UiText,
                Text::new("Basic Training"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Body),
            )],
        ));
        if catalog.is_empty() {
            list.spawn((
                Name::new("Training Empty Note"),
                UiText,
                Text::new("No lessons installed."),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
            ));
            return;
        }
        for (category, lessons) in catalog.by_category() {
            list.spawn((
                Name::new(format!("Training Category: {}", category.title())),
                Node {
                    margin: UiRect::top(px(10)),
                    ..default()
                },
                children![(
                    Name::new("Training Category Label"),
                    panel_header(category.title())
                )],
            ));
            for lesson in lessons {
                spawn_lesson_row(
                    list,
                    lesson,
                    progress.status(&lesson.id),
                    selected.0.as_deref() == Some(lesson.id.as_str()),
                );
            }
        }
    });
}

/// One lesson row: the lesson's title, with the state badge on the right.
///
/// A title and nothing else. The list is the handbook's table of contents, and
/// the tip itself is one screen away - a per-row blurb here would be the tip
/// said twice, in the smaller of the two places.
fn spawn_lesson_row(
    list: &mut ChildSpawnerCommands,
    lesson: &Lesson,
    status: LessonStatus,
    selected: bool,
) {
    let mut row = list.spawn((
        Name::new(format!("Lesson Row: {}", lesson.id)),
        LessonRow {
            id: lesson.id.clone(),
        },
        list_row(),
        Button,
        Hovered::default(),
        observe(on_lesson_row_select),
    ));
    if selected {
        row.insert(Selected);
    }
    row.with_children(|row| {
        row.spawn((
            Name::new("Lesson Title"),
            UiText,
            Text::new(lesson.title.clone()),
            TextFont {
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Body),
            Node {
                flex_grow: 1.0,
                // Without this a long title refuses to wrap and pushes the
                // badge off the pane at the narrow window.
                min_width: px(0),
                ..default()
            },
        ));
        if let Some((kind, text)) = status_badge(status) {
            row.spawn((
                Name::new(format!("Lesson Status: {}", lesson.id)),
                LessonStatusBadge(status),
                badge(kind, text),
            ));
        }
    });
}

/// The badge a row's state draws, or `None` for a lesson nobody has opened -
/// an untouched list should read as a list, not as a column of tags.
fn status_badge(status: LessonStatus) -> Option<(BadgeKind, &'static str)> {
    match status {
        LessonStatus::New => None,
        LessonStatus::Viewed => Some((BadgeKind::Mute, "read")),
        LessonStatus::Completed => Some((BadgeKind::Green, "done")),
    }
}

/// Rebuild the details pane from the selected lesson.
///
/// One tip, one screen: the demonstration takes the width of the pane at 16:9,
/// and the text box under it holds everything else - the title, the whole body,
/// the live binding chips and the way on. There is no page cursor, because
/// there are no pages.
#[expect(
    clippy::too_many_arguments,
    reason = "the pane draws from the catalog, the record, the selection, the binding table and               the asset stack; splitting it would only move the argument list"
)]
pub(crate) fn refresh_training_details(
    mut commands: Commands,
    catalog: Res<TrainingCatalog>,
    progress: Res<TrainingProgress>,
    selected: Res<SelectedLessonId>,
    bindings: Option<Res<InputBindings>>,
    hud_assets: Option<Res<NovaHudAssets>>,
    asset_server: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    mut pending: ResMut<PendingLessonMedia>,
    panels: Query<Entity, With<TrainingDetailsPanel>>,
) {
    let Ok(pane) = panels.single() else {
        return;
    };
    commands.entity(pane).despawn_related::<Children>();
    let lesson = selected
        .0
        .as_deref()
        .and_then(|id| catalog.get(id))
        .cloned();
    // The keycap art lives inside the HUD's asset bundle, the same place
    // the settings rebind rows read it from. A rig without it falls back to
    // the chip's own text.
    let glyphs = hud_assets.as_deref().map(|assets| &assets.key_glyphs);
    let bindings = bindings.as_deref();
    let status = lesson
        .as_ref()
        .and_then(|lesson| status_badge(progress.status(&lesson.id)));
    // Resolved BEFORE the pane is built: cutting a sheet into cells needs a
    // mutable atlas-layout store, which a child-spawning closure cannot hold.
    // `None` is the fallback frame, and says nothing about why - a missing
    // asset stack, an image still loading, and a texture the UI cannot bind all
    // leave the lesson readable.
    pending.0 = None;
    let media = lesson.as_ref().and_then(|lesson| {
        resolve_media(
            &lesson.media,
            asset_server.as_deref(),
            images.as_deref(),
            layouts,
            &mut pending,
        )
    });

    commands.entity(pane).with_children(|details| {
        let Some(lesson) = lesson else {
            details.spawn((
                Name::new("Training Details Empty"),
                UiText,
                Text::new("Select a lesson."),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
            ));
            return;
        };

        spawn_media_frame(details, &lesson.media, media);

        details
            .spawn((
                Name::new("Lesson Text Box"),
                Node {
                    // Takes whatever the 16:9 frame leaves: about a quarter of
                    // the pane on a desk monitor, about half at the 640x600
                    // floor. The FRAME keeps its shape on every machine, so an
                    // authored loop is drawn the same everywhere and the box is
                    // what gives.
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    padding: UiRect::all(px(12)),
                    margin: UiRect::top(px(10)),
                    border: UiRect::all(px(theme::BORDER_W)),
                    ..default()
                },
                ThemedRadius::control(),
                BorderColor::all(Color::NONE),
                ThemedBorder::new(UiColor::Label),
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColor::Surface),
            ))
            .with_children(|box_| {
                box_.spawn((
                    Name::new("Lesson Heading"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                        ..default()
                    },
                ))
                .with_children(|head| {
                    head.spawn((
                        Name::new("Lesson Details Title"),
                        UiText,
                        Text::new(lesson.title.clone()),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Body),
                        Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            ..default()
                        },
                    ));
                    if let Some((kind, text)) = status {
                        head.spawn((Name::new("Lesson Details Status"), badge(kind, text)));
                    }
                });

                // The body scrolls and the action strip below it does not, so
                // the way on stays on screen even if a lesson is written over
                // the box it was measured for.
                box_.spawn((Name::new("Lesson Body Row"), scroll_row()))
                    .with_children(|row| {
                        row.spawn((Name::new("Lesson Body"), scroll_column(), scroll_viewport()))
                            .with_children(|body| {
                                body.spawn((
                                    Name::new("Lesson Body Text"),
                                    UiText,
                                    Text::new(lesson.body.clone()),
                                    TextFont {
                                        font_size: FontSize::Px(14.0),
                                        ..default()
                                    },
                                    TextColor(Color::NONE),
                                    ThemedText::new(UiColor::Body),
                                ));
                                if !lesson.actions.is_empty() {
                                    body.spawn((
                                        Name::new("Lesson Bindings"),
                                        Node {
                                            flex_direction: FlexDirection::Column,
                                            margin: UiRect::top(px(8)),
                                            ..default()
                                        },
                                    ))
                                    .with_children(|list| {
                                        for action in &lesson.actions {
                                            spawn_binding_row(list, action, bindings, glyphs);
                                        }
                                    });
                                }
                            });
                        row.spawn((Name::new("Lesson Body Scroll Bar"), scroll_bar()));
                    });

                spawn_lesson_actions(box_, &lesson);
            });
    });
}

/// A lesson's demonstration, loaded and ready to mount.
enum ResolvedMedia {
    /// One still frame.
    Still(Handle<Image>),
    /// A sprite sheet, its cells already laid out.
    Loop {
        sheet: Handle<Image>,
        layout: Handle<TextureAtlasLayout>,
        frames: u32,
        seconds_per_frame: f32,
    },
}

/// Turn an authored media ref into something mountable, or `None`.
///
/// Three guards, all of which fall back rather than fail. The asset stack may
/// be absent (headless rigs have no `AssetServer`). The image may still be
/// loading, in which case the handle is parked in [`PendingLessonMedia`] so the
/// refresh re-runs when it lands. And the image must be a plain 2D single-layer
/// texture: the UI pipeline binds a D2 texture, so a cube or array image - a
/// lesson pointed at a skybox, say - would make wgpu reject the bind group,
/// which is a hard render crash on native AND web.
fn resolve_media(
    media: &LessonMedia,
    asset_server: Option<&AssetServer>,
    images: Option<&Assets<Image>>,
    mut layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    pending: &mut PendingLessonMedia,
) -> Option<ResolvedMedia> {
    let server = asset_server?;
    let handle = media.source().resolve(server);
    if !server.is_loaded_with_dependencies(&handle) {
        pending.0 = Some(handle);
        return None;
    }
    let image = images?.get(&handle)?;
    let size = image.texture_descriptor.size;
    if size.depth_or_array_layers != 1 {
        warn!(
            "a lesson demonstration is not a 2D image (cube/array/3D texture); drawing the \
             fallback instead - handbook media must be a plain 2D image."
        );
        return None;
    }
    match media {
        LessonMedia::Image { .. } => Some(ResolvedMedia::Still(handle)),
        LessonMedia::Loop {
            columns,
            rows,
            frames,
            frames_per_second,
            ..
        } => {
            // The grid is AUTHORED and the sheet is just pixels, so a grid the
            // image cannot be cut by is an authoring error the content lint
            // reports. Here it degrades to the fallback rather than drawing a
            // sliver of somebody else's frame.
            if *columns == 0 || *rows == 0 || size.width % columns != 0 || size.height % rows != 0 {
                warn!(
                    "a lesson loop authors a {columns}x{rows} grid over a {}x{} sheet, which \
                     does not divide evenly; drawing the fallback instead.",
                    size.width, size.height
                );
                return None;
            }
            let cell = UVec2::new(size.width / columns, size.height / rows);
            let layout = layouts.as_mut()?.add(TextureAtlasLayout::from_grid(
                cell, *columns, *rows, None, None,
            ));
            Some(ResolvedMedia::Loop {
                sheet: handle,
                layout,
                frames: (*frames).min(columns * rows),
                seconds_per_frame: 1.0 / frames_per_second.max(1.0),
            })
        }
    }
}

/// The media frame: a 16:9 box carrying the demonstration, or - while it loads,
/// or when it cannot be drawn - the alternate text that says what is in it and
/// a tag saying which FORM.
///
/// The frame keeps its shape either way. That is the point of it: the text box
/// below takes whatever is left, so a lesson whose art is still arriving does
/// not shove the prose up the pane and then back down again.
fn spawn_media_frame(
    body: &mut ChildSpawnerCommands,
    media: &LessonMedia,
    resolved: Option<ResolvedMedia>,
) {
    let (kind, tag) = if media.is_loop() {
        (BadgeKind::Amber, "loop")
    } else {
        (BadgeKind::Blue, "still")
    };
    let alt = media.alt().to_string();
    body.spawn((
        Name::new("Lesson Media Frame"),
        Node {
            width: percent(100),
            flex_shrink: 0.0,
            aspect_ratio: Some(16.0 / 9.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(px(12)),
            row_gap: px(8),
            border: UiRect::all(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Label),
        BackgroundColor(Color::NONE),
        ThemedFill::new(UiColor::Surface),
    ))
    .with_children(|frame| match resolved {
        Some(ResolvedMedia::Still(handle)) => {
            frame.spawn((
                Name::new("Lesson Media Image"),
                ImageNode::new(handle),
                media_node(),
            ));
        }
        Some(ResolvedMedia::Loop {
            sheet,
            layout,
            frames,
            seconds_per_frame,
        }) => {
            frame.spawn((
                Name::new("Lesson Media Loop"),
                ImageNode::from_atlas_image(sheet, TextureAtlas { layout, index: 0 }),
                media_node(),
                LessonLoop {
                    frames,
                    timer: Timer::from_seconds(seconds_per_frame, TimerMode::Repeating),
                },
            ));
        }
        None => {
            frame.spawn((Name::new("Lesson Media Tag"), badge(kind, tag)));
            frame.spawn((
                Name::new("Lesson Media Legend"),
                UiText,
                Text::new(alt),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Secondary),
                TextLayout {
                    justify: Justify::Center,
                    ..default()
                },
            ));
        }
    });
}

/// How a mounted demonstration fills its frame: the whole box, letterboxed
/// rather than cropped, so an authored sheet that is not 16:9 is drawn whole.
fn media_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        ..default()
    }
}

/// While the selected lesson's demonstration is still loading (parked in
/// [`PendingLessonMedia`] by `refresh_training_details`), re-arm that refresh
/// the moment it lands, so it can validate the texture and mount the image.
/// Without this a lesson opened before its art arrived would keep the fallback
/// until the player clicked another row.
pub(crate) fn poll_lesson_media(
    pending: Res<PendingLessonMedia>,
    asset_server: Option<Res<AssetServer>>,
    mut selected: ResMut<SelectedLessonId>,
) {
    let (Some(handle), Some(server)) = (pending.0.as_ref(), asset_server.as_ref()) else {
        return;
    };
    if server.is_loaded_with_dependencies(handle) {
        // `training_details_dirty` keys off `selected.is_changed()`; re-running
        // the refresh mounts the now-loaded image and clears `pending`.
        selected.set_changed();
    }
}

/// Step every running demonstration to its next cell.
///
/// `Time<Real>` on purpose: this is menu chrome, and the menu's virtual clock
/// is held while a modal is open. A loop that stopped when the player opened
/// the handbook would be a loop nobody ever sees run.
pub(crate) fn advance_lesson_loops(
    time: Res<Time<Real>>,
    mut loops: Query<(&mut LessonLoop, &mut ImageNode)>,
) {
    for (mut running, mut node) in &mut loops {
        if running.frames == 0 {
            continue;
        }
        running.timer.tick(time.delta());
        let steps = running.timer.times_finished_this_tick();
        if steps == 0 {
            continue;
        }
        if let Some(atlas) = node.texture_atlas.as_mut() {
            atlas.index = (atlas.index + steps as usize) % running.frames as usize;
        }
    }
}

/// One action's live binding row: its registry LABEL, then its current chips.
///
/// The label and the keys both come from [`InputBindings`], so a rebind moves
/// them and no lesson has ever written a key down.
fn spawn_binding_row(
    body: &mut ChildSpawnerCommands,
    action: &str,
    bindings: Option<&InputBindings>,
    glyphs: Option<&KeyGlyphs>,
) {
    let binding = bindings.and_then(|bindings| bindings.get(action));
    let label = binding.map_or_else(
        // An action the registry does not know is an authoring error, and it
        // says so on the screen rather than drawing a blank row. Nothing
        // rejects it earlier: the screen is the only thing that catches it.
        || format!("unknown action `{action}`"),
        |binding| binding.label.to_string(),
    );
    let chips = binding
        .map(|binding| binding.keyboard_chips())
        .unwrap_or_default();
    body.spawn((
        Name::new(format!("Lesson Binding: {action}")),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            margin: UiRect::bottom(px(6)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            UiText,
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Body),
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        ));
        row.spawn((
            Name::new("Lesson Binding Chips"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
        ))
        .with_children(|slot| {
            spawn_binding_chips(slot, &chips, glyphs, UiColor::Body, None);
        });
    });
}

/// The strip along the bottom of the text box: the way on, and where the
/// complete manual continues.
///
/// Inside the box rather than under it, because the box is the lesson's one
/// piece of furniture and a button floating below it would read as belonging to
/// the screen instead of to the tip.
fn spawn_lesson_actions(box_: &mut ChildSpawnerCommands, lesson: &Lesson) {
    box_.spawn((
        Name::new("Lesson Actions"),
        Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            // The wiki link is anchored to the RIGHT edge either way. With
            // `SpaceBetween` alone a lesson without a Practice button would be
            // the only child and slide to the left, so the link would move
            // across the box as the player walks the list.
            justify_content: if lesson.practice.is_some() {
                JustifyContent::SpaceBetween
            } else {
                JustifyContent::FlexEnd
            },
            column_gap: px(8),
            row_gap: px(6),
            margin: UiRect::top(px(6)),
            ..default()
        },
    ))
    .with_children(|actions| {
        if let Some(scenario) = &lesson.practice {
            actions.spawn((
                Name::new("Lesson Practice Button"),
                nova_ui::widget::button(ButtonSpec::new(PRACTICE_LABEL).primary().fit()),
                crate::widgets::MenuSfxButton,
                LessonPractice {
                    scenario: scenario.clone(),
                },
                observe(on_lesson_practice),
            ));
        }
        // The PAGE, not the anchor. A lesson links to the heading its claim
        // sits under, and that path has no spaces in it, so the whole
        // `wiki/flight-autopilot#the-autopilot-flies-the-hull` is one
        // unbreakable word that would run out of the box at the narrow floor.
        // The page is also what a player would type.
        let page = lesson
            .wiki_path
            .split_once('#')
            .map_or(lesson.wiki_path.as_str(), |(page, _)| page);
        actions.spawn((
            Name::new("Lesson Wiki Link"),
            UiText,
            Text::new(format!("Full manual: {page}")),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Label),
        ));
    });
}

/// Keep the notice corner out of the way of whatever modal is open, and take
/// each notice off it once the player has switched that notice off.
///
/// THREE decisions, because the corner holds three different kinds of thing.
/// The CORNER is about containment, not decoration: the modals are 85 percent
/// of the window, so a card at the screen edge would otherwise still be visible
/// and CLICKABLE beside a screen the player thinks owns the input. The PROMPT
/// is about the offer: answering it is permanent, and the setting is what
/// remembers. The NOTE is not an offer - it outlives the answer to the offer -
/// but it has a switch of its own, so it reads its own setting.
///
/// Losing the corner loses nothing: `Lessons` in the menu card is the
/// permanent way into the handbook, and Settings > Interface brings both
/// notices back.
pub(crate) fn sync_menu_aside(
    prompt: Res<TrainingPromptSetting>,
    field_note: Res<FieldNoteSetting>,
    // The `Without`s are what make the queries provably disjoint: all four
    // read `Visibility`, and three of them write it.
    panels: Query<
        &Visibility,
        (
            Or<(
                With<TrainingPanel>,
                With<SettingsPanel>,
                With<ModsPanel>,
                With<ScenariosPanel>,
            )>,
            Without<MenuAside>,
            Without<TrainingPromptCard>,
            Without<MenuFieldNoteCard>,
        ),
    >,
    mut aside: Query<
        &mut Visibility,
        (
            With<MenuAside>,
            Without<TrainingPromptCard>,
            Without<MenuFieldNoteCard>,
        ),
    >,
    mut offer: Query<&mut Visibility, (With<TrainingPromptCard>, Without<MenuFieldNoteCard>)>,
    mut note: Query<&mut Visibility, With<MenuFieldNoteCard>>,
) {
    let modal_open = panels
        .iter()
        .any(|visibility| *visibility == Visibility::Visible);
    set_all(
        &mut aside,
        if modal_open {
            Visibility::Hidden
        } else {
            Visibility::Visible
        },
    );
    // Inherited, not Visible: a notice is only ever on screen when the corner
    // holding it is, and a modal must not leave one card behind.
    set_all(
        &mut offer,
        if prompt.shown() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
    );
    set_all(
        &mut note,
        if field_note.shown() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
    );
}

/// Write `wanted` to every match, and only where it is not already there.
fn set_all<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<&mut Visibility, F>,
    wanted: Visibility,
) {
    for mut visibility in query {
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}
