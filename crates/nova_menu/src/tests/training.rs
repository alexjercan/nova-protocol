//! The Training handbook screen: what the list draws, what selecting a lesson
//! claims, that one tip is one screen, that a binding chip follows a rebind,
//! and that the first-launch prompt in the corner is answered once and gone.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_input::prelude::{BindingSpec, InputBindings, InputSource};
use nova_training::prelude::*;
use nova_ui::prelude::Selected;
use nova_world_base::prelude::OpenWorldSession;

use super::support::{
    all_texts, app, dummy_lessons, dummy_progress, dummy_scenarios, entity_by_name, FIXTURE_NOTE,
    FIXTURE_NOTE_LESSON, TEST_RANGE_ID,
};
use crate::{
    scenarios::NewGameScenario,
    settings::{FieldNoteSetting, TrainingPromptSetting},
    training::{
        LessonRow, LessonStatusBadge, MenuAside, MenuFieldNoteCard, SelectedLessonId,
        TrainingPanel, TrainingPromptCard,
    },
    training_store::TrainingStoreAccess,
    world_setup::WorldSetupOverlay,
};

/// A menu app sitting on the front door with the handbook built.
///
/// A fresh install, so the corner prompt is up: `TrainingPromptSetting`
/// defaults to `Shown` and the test store never wrote anything else.
fn training_app() -> App {
    training_app_with(|_| {})
}

/// The same menu, with one more thing arranged BEFORE the front door is
/// entered - the corner is built on the way in and never reconciled, so a
/// resource a test wants the build to read has to be there first.
fn training_app_with(arrange: impl FnOnce(&mut App)) -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    arrange(&mut app);
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    // One more frame so the list's default selection reaches the details pane.
    app.update();
    app
}

/// Every `Text` on an entity carrying `name`, in world order.
fn texts_named(app: &mut App, name: &str) -> Vec<String> {
    let mut q = app.world_mut().query::<(&Name, &Text)>();
    q.iter(app.world())
        .filter(|(n, _)| n.as_str() == name)
        .map(|(_, text)| text.0.clone())
        .collect()
}

/// The same menu for a player who answered the offer on an earlier launch:
/// the setting says `Hidden`, which is what a loaded settings file would say.
fn answered_prompt_app() -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(TrainingPromptSetting::Hidden);
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app.update();
    app
}

fn click(app: &mut App, name: &str) {
    let entity = entity_by_name(app, name).unwrap_or_else(|| panic!("no `{name}` on screen"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn visibility(app: &mut App, name: &str) -> Visibility {
    let entity = entity_by_name(app, name).unwrap_or_else(|| panic!("no `{name}` on screen"));
    *app.world().get::<Visibility>(entity).expect("a Visibility")
}

fn selected(app: &App) -> Option<String> {
    app.world().resource::<SelectedLessonId>().0.clone()
}

fn status(app: &App, id: &str) -> LessonStatus {
    app.world().resource::<TrainingProgress>().status(id)
}

/// Every lesson row, in the order the list DRAWS them.
///
/// Read off `Children`, not off a query: query iteration is archetype order,
/// and the selected row carries an extra `Selected` component, so a query would
/// report the one row the player is looking at in the wrong place.
fn rows(app: &mut App) -> Vec<String> {
    let list = entity_by_name(app, "Training List").expect("the list");
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter_map(|child| {
            app.world()
                .get::<LessonRow>(child)
                .map(|row| row.id.clone())
        })
        .collect()
}

/// Every lesson row ENTITY, in draw order - the identity the rebuild tests read.
fn row_entities(app: &mut App) -> Vec<Entity> {
    let list = entity_by_name(app, "Training List").expect("the list");
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter(|child| app.world().get::<LessonRow>(*child).is_some())
        .collect()
}

/// The id of the row wearing the `Selected` highlight.
fn marked_row(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<&LessonRow, With<Selected>>();
    q.iter(app.world()).next().map(|row| row.id.clone())
}

/// The badge legends on one row.
fn badge_texts(app: &mut App, row: Entity) -> Vec<String> {
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(row)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter(|child| app.world().get::<LessonStatusBadge>(*child).is_some())
        .flat_map(|badge| {
            let grandchildren: Vec<Entity> = app
                .world()
                .get::<Children>(badge)
                .map(|children| children.iter().collect())
                .unwrap_or_default();
            grandchildren
                .into_iter()
                .filter_map(|child| app.world().get::<Text>(child).map(|text| text.0.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn the_list_draws_after_a_boot_that_ran_frames_before_the_menu() {
    // The shipped boot order, which every other fixture here skips: the app
    // runs `Update` through `Loading` while the assets come in, and only then
    // enters the menu. That first `Update` is where the schedule initializes
    // its systems and conditions, so the catalog and the progress record -
    // inserted when the plugin was BUILT - are already old news by the time the
    // handbook's refreshers are first asked whether they have work. Without the
    // just-spawned trigger in `training_list_dirty` this app opens a handbook
    // with no lessons in it, no details, a blank progress line and an empty
    // corner card, and every other test here still passes.
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app.update();

    assert!(
        !rows(&mut app).is_empty(),
        "the handbook opened with an empty lesson list after a normal boot"
    );
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|text| text.contains("completed")),
        "the progress summary never got written: {texts:?}"
    );
    assert!(
        entity_by_name(&mut app, "Lesson Details Title").is_some(),
        "the details pane is empty after a normal boot"
    );
    assert!(
        entity_by_name(&mut app, "Lessons Button").is_some(),
        "the menu card has no way into the handbook after a normal boot"
    );
}

/// The permanent way in is the menu card's own row, so a player who answered
/// the corner prompt on their first launch can still read the handbook.
#[test]
fn the_lessons_row_opens_the_handbook_and_back_closes_it() {
    let mut app = answered_prompt_app();
    assert_eq!(
        visibility(&mut app, "Training Panel Root"),
        Visibility::Hidden,
        "the handbook is closed until it is asked for"
    );

    click(&mut app, "Lessons Button");
    assert_eq!(
        visibility(&mut app, "Training Panel Root"),
        Visibility::Visible
    );

    click(&mut app, "Training Back Button");
    assert_eq!(
        visibility(&mut app, "Training Panel Root"),
        Visibility::Hidden
    );
}

/// The list is the catalog's own order, and every category that holds a lesson
/// draws its header. The prototype fills all six, which is the point: the
/// layout is reviewed against the real information architecture.
#[test]
fn the_list_draws_every_category_and_every_lesson() {
    let mut app = training_app();
    let catalog = app.world().resource::<TrainingCatalog>().clone();
    let expected: Vec<String> = catalog.lessons().iter().map(|l| l.id.clone()).collect();
    assert_eq!(rows(&mut app), expected);

    let texts = all_texts(&mut app);
    for (category, _) in catalog.by_category() {
        assert!(
            texts.iter().any(|t| t == &category.title().to_uppercase()),
            "no header for {}: {texts:?}",
            category.title()
        );
    }
}

/// Opening the handbook puts a lesson on screen without the player choosing it.
/// That is not reading, and the record must not say it was.
#[test]
fn the_default_selection_does_not_claim_the_player_read_anything() {
    let app = training_app();
    let first = app
        .world()
        .resource::<TrainingCatalog>()
        .first_id()
        .cloned()
        .expect("a first lesson");
    assert_eq!(selected(&app).as_deref(), Some(first.as_str()));

    // `start_welcome` is seeded Completed by the prototype, so this asks the
    // question on a lesson the seed leaves alone.
    let untouched = "combat_turrets";
    assert_eq!(status(&app, untouched), LessonStatus::New);
    assert!(
        app.world()
            .resource::<TrainingProgress>()
            .viewed()
            .all(|id| id != untouched),
        "nothing marked a lesson nobody opened"
    );
}

#[test]
fn clicking_a_lesson_marks_it_viewed_and_never_completed() {
    let mut app = training_app();
    assert_eq!(status(&app, "combat_turrets"), LessonStatus::New);

    click(&mut app, "Lesson Row: combat_turrets");

    assert_eq!(selected(&app).as_deref(), Some("combat_turrets"));
    assert_eq!(
        status(&app, "combat_turrets"),
        LessonStatus::Viewed,
        "reading a tip is all reading a tip can claim"
    );
}

/// Viewed and Completed are two different words on the screen, and a lesson
/// nobody has opened wears neither.
#[test]
fn the_row_states_read_apart() {
    let mut app = training_app();
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "[DONE]"),
        "the completed lesson has no badge: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "[READ]"),
        "the viewed lesson has no badge: {texts:?}"
    );
    // start_welcome is done, start_hud and flight_momentum are read; every
    // other prototype lesson is untouched, and an untouched list reads as a
    // list.
    let badges = texts.iter().filter(|t| t.starts_with('[')).count();
    let lessons = app.world().resource::<TrainingCatalog>().len();
    assert!(
        badges < lessons,
        "every row wears a badge; a new lesson should wear none"
    );
}

/// The record the handbook draws is the FIXTURE's, not whatever the last test
/// run left on disk.
///
/// `NovaMenuPlugin` brings a store that loads at startup and saves on change,
/// and every menu test shares one root. Left live, a row click in one test wrote
/// a file the next run loaded over `dummy_progress`, and the counts below drifted
/// with whatever the suite happened to do last - which is how this was found.
#[test]
fn the_fixture_record_is_not_a_file_an_earlier_run_left() {
    let app = training_app();
    assert_eq!(
        *app.world().resource::<TrainingStoreAccess>(),
        TrainingStoreAccess::Inert,
        "the menu fixture must neither load nor save a progress file"
    );
    assert_eq!(
        *app.world().resource::<TrainingProgress>(),
        dummy_progress(),
        "the handbook must draw the fixture record it was handed"
    );
}

#[test]
fn the_progress_summary_counts_completed_and_opened() {
    let mut app = training_app();
    let total = app.world().resource::<TrainingCatalog>().len();
    let texts = all_texts(&mut app);
    let wanted = format!("1 of {total} completed - 3 opened");
    assert!(
        texts.iter().any(|t| t == &wanted),
        "no progress summary reading `{wanted}`: {texts:?}"
    );
}

/// ONE TIP, ONE SCREEN. There is no page cursor and no Prev/Next, so the whole
/// body is on the screen the moment the row is clicked - not behind a control
/// the player has to find.
#[test]
fn a_lesson_puts_its_whole_body_on_one_screen() {
    let mut app = training_app();
    click(&mut app, "Lesson Row: flight_momentum");
    let body = app
        .world()
        .resource::<TrainingCatalog>()
        .get("flight_momentum")
        .expect("the lesson")
        .body
        .clone();

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == &body),
        "the tip's body is not on the screen in one piece: {texts:?}"
    );
    for name in ["Lesson Page Next", "Lesson Page Prev", "Lesson Page Count"] {
        assert!(
            entity_by_name(&mut app, name).is_none(),
            "`{name}` is back: one tip is one screen"
        );
    }
}

/// The frame is a fixed part of the layout, not something that appears when a
/// lesson happens to carry art. A frame that came and went would move the text
/// box up and down the pane as the player walked the list.
#[test]
fn every_lesson_draws_its_media_frame_and_its_text_box() {
    let mut app = training_app();
    let ids: Vec<String> = app
        .world()
        .resource::<TrainingCatalog>()
        .lessons()
        .iter()
        .map(|lesson| lesson.id.clone())
        .collect();
    for id in ids {
        click(&mut app, &format!("Lesson Row: {id}"));
        assert!(
            entity_by_name(&mut app, "Lesson Media Frame").is_some(),
            "{id} drew no media frame"
        );
        assert!(
            entity_by_name(&mut app, "Lesson Text Box").is_some(),
            "{id} drew no text box"
        );
    }
}

/// The strip names the PAGE the manual continues on. A lesson links to the
/// heading its claim sits under, but that path is one unbreakable word with no
/// spaces to wrap at, so the anchor would run out of the box at the narrow
/// floor - and the page is what a player would type anyway.
#[test]
fn the_wiki_line_names_the_page_without_its_heading_anchor() {
    let mut app = training_app();
    click(&mut app, "Lesson Row: flight_momentum");

    let path = app
        .world()
        .resource::<TrainingCatalog>()
        .get("flight_momentum")
        .expect("the lesson")
        .wiki_path
        .clone();
    let (page, anchor) = path.split_once('#').expect("the fixture anchors its path");

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == &format!("Full manual: {page}")),
        "the strip does not name the page: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains(anchor)),
        "the heading anchor is on screen: {texts:?}"
    );
}

/// The lesson names the ACTION. What the screen prints is whatever the player
/// has that action bound to right now - including after they rebind it.
#[test]
fn a_binding_chip_says_what_the_action_is_bound_to_now() {
    let mut app = training_app();
    click(&mut app, "Lesson Row: novaos_open");

    let label = app
        .world()
        .resource::<InputBindings>()
        .get("novaos_toggle")
        .expect("the action is registered")
        .label
        .to_string();
    assert!(
        all_texts(&mut app).iter().any(|t| t == &label),
        "the lesson does not name the action by its registry label"
    );

    let rebound = app.world_mut().resource_mut::<InputBindings>().rebind(
        "novaos_toggle",
        BindingSpec {
            keyboard: vec![InputSource::Keyboard(KeyCode::F9)],
            gamepad: vec![],
        },
    );
    assert!(rebound, "the fixture could not rebind the action");
    app.update();

    assert!(
        all_texts(&mut app).iter().any(|t| t.contains("F9")),
        "the chip still shows the old key after a rebind: {:?}",
        all_texts(&mut app)
    );
}

/// `Start Basic Training` plays Basic Training, whatever the picker held and
/// whatever the bundle declares New Game to start.
#[test]
fn start_training_plays_basic_training_and_answers_the_offer() {
    let mut app = training_app();
    app.world_mut().resource_mut::<NewGameScenario>().0 = Some("something_else".to_string());

    click(&mut app, "Training Prompt Start");

    assert_eq!(
        app.world().resource::<NewGameScenario>().0.as_deref(),
        Some(TUTORIAL_SCENARIO_ID)
    );
    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    assert_eq!(
        *app.world().resource::<TrainingPromptSetting>(),
        TrainingPromptSetting::Hidden,
        "taking the offer must not offer it again on the next launch"
    );
}

#[test]
fn open_lessons_opens_the_handbook_at_the_first_lesson() {
    let mut app = training_app();
    click(&mut app, "Lesson Row: advanced_mods");
    click(&mut app, "Training Back Button");

    click(&mut app, "Training Prompt Lessons");

    assert_eq!(
        visibility(&mut app, "Training Panel Root"),
        Visibility::Visible
    );
    let first = app
        .world()
        .resource::<TrainingCatalog>()
        .first_id()
        .cloned();
    assert_eq!(selected(&app), first);
    assert_eq!(
        *app.world().resource::<TrainingPromptSetting>(),
        TrainingPromptSetting::Shown,
        "reading is not answering the offer"
    );
}

/// Every Practice button says the same word, and it hands off through the SAME
/// New Game route the Scenarios picker uses - a second launch path is exactly
/// what the handbook must not grow.
#[test]
fn practice_launches_the_lessons_own_range_through_new_game() {
    let mut app = training_app();
    click(&mut app, "Lesson Row: combat_turrets");

    let labels = all_texts(&mut app);
    assert!(
        labels.iter().any(|t| t == "Practice"),
        "the practice button wears the screen's one common label: {labels:?}"
    );

    click(&mut app, "Lesson Practice Button");

    assert_eq!(
        app.world().resource::<NewGameScenario>().0.as_deref(),
        Some(TEST_RANGE_ID),
        "Practice hands the range to the same pick New Game reads"
    );
    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    assert_eq!(
        status(&app, "combat_turrets"),
        LessonStatus::Viewed,
        "flying the range is not proof of anything; only reading was claimed"
    );
}

/// New Game opens the open world, so the handbook heads its list with Basic
/// Training for a player who has answered the corner offer. The row launches
/// straight into the course: no world setup modal and no open-world session.
#[test]
fn the_first_lessons_row_plays_basic_training_without_the_world_setup() {
    let mut app = answered_prompt_app();
    app.world_mut().resource_mut::<NewGameScenario>().0 = Some("something_else".to_string());
    click(&mut app, "Lessons Button");

    let list = entity_by_name(&mut app, "Training List").expect("the list");
    let first = app.world().get::<Children>(list).expect("list children")[0];
    assert_eq!(
        entity_by_name(&mut app, "Training Basic Training"),
        Some(first),
        "Basic Training heads the list, above every category"
    );
    assert_eq!(
        texts_named(&mut app, "Training Basic Training Title"),
        ["Basic Training"]
    );

    click(&mut app, "Training Basic Training");

    assert_eq!(
        app.world().resource::<NewGameScenario>().0.as_deref(),
        Some(TUTORIAL_SCENARIO_ID)
    );
    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    let mut overlays = app
        .world_mut()
        .query_filtered::<(), With<WorldSetupOverlay>>();
    assert_eq!(overlays.iter(app.world()).count(), 0);
    assert!(app.world().get_resource::<OpenWorldSession>().is_none());
}

/// A lesson with nothing focused to fly offers no button at all, rather than
/// the nearest approximation.
#[test]
fn a_lesson_with_no_range_offers_no_practice_button() {
    let mut app = training_app();
    click(&mut app, "Lesson Row: advanced_mods");
    assert!(entity_by_name(&mut app, "Lesson Practice Button").is_none());
}

/// A fresh install meets the offer once.
#[test]
fn a_fresh_install_gets_the_offer() {
    let mut app = training_app();
    assert_eq!(visibility(&mut app, "Menu Aside"), Visibility::Visible);
    assert!(entity_by_name(&mut app, "Training Prompt Start").is_some());
    assert!(entity_by_name(&mut app, "Training Prompt Dismiss").is_some());
}

/// `Not now` answers the offer for good, and the answer is a SETTING - the
/// same place the player can switch it back on. The CORNER stays: it is a
/// notice centre, and the field note in it was never an offer to answer.
#[test]
fn not_now_takes_the_offer_down_for_good() {
    let mut app = training_app();

    click(&mut app, "Training Prompt Dismiss");

    assert_eq!(
        *app.world().resource::<TrainingPromptSetting>(),
        TrainingPromptSetting::Hidden
    );
    assert_eq!(visibility(&mut app, "Training Prompt"), Visibility::Hidden);
    assert_eq!(
        visibility(&mut app, "Menu Aside"),
        Visibility::Visible,
        "the corner still holds the field note"
    );
    // And nothing was lost: the handbook is still one click away.
    click(&mut app, "Lessons Button");
    assert_eq!(
        visibility(&mut app, "Training Panel Root"),
        Visibility::Visible
    );
}

/// The player who switched the prompt back on in Settings gets it back on the
/// next menu entry, which is the whole point of putting it there.
///
/// `Inherited`, not `Visible`: the corner decides whether anything in it is on
/// screen, and a card that asserted itself Visible would survive a modal.
#[test]
fn the_setting_is_what_decides_whether_the_offer_draws() {
    let mut app = answered_prompt_app();
    assert_eq!(visibility(&mut app, "Training Prompt"), Visibility::Hidden);

    *app.world_mut().resource_mut::<TrainingPromptSetting>() = TrainingPromptSetting::Shown;
    app.update();

    assert_eq!(
        visibility(&mut app, "Training Prompt"),
        Visibility::Inherited
    );
}

/// The corner's second notice: one fact, read off the catalog rather than
/// written on the card, so the claim and the lesson cannot drift apart.
#[test]
fn the_corner_draws_a_field_note_from_the_catalog() {
    let mut app = training_app();

    let drawn = texts_named(&mut app, "Menu Field Note Line");
    assert!(
        drawn.iter().any(|text| text == FIXTURE_NOTE),
        "the authored note must be on the card: {drawn:?}"
    );
}

/// The note is a way IN: it names the lesson that owns the claim, and opening
/// it from here is a choice, so it counts as reading.
#[test]
fn the_note_opens_the_lesson_it_came_from() {
    let mut app = training_app();
    assert_eq!(
        status(&app, FIXTURE_NOTE_LESSON),
        LessonStatus::New,
        "delivery guard: the fixture has no history for this lesson"
    );

    click(&mut app, "Menu Field Note Lesson Button");

    assert_eq!(
        visibility(&mut app, "Training Panel Root"),
        Visibility::Visible
    );
    assert_eq!(selected(&app), Some(FIXTURE_NOTE_LESSON.to_string()));
    assert_eq!(status(&app, FIXTURE_NOTE_LESSON), LessonStatus::Viewed);
}

/// A build whose catalog carries no note at all draws the corner it drew before
/// there were any, rather than an empty card.
#[test]
fn a_catalog_with_no_notes_draws_no_note_card() {
    let mut app = training_app_with(|app| {
        let bare =
            TrainingCatalog::new(dummy_lessons().lessons().iter().cloned().map(|mut lesson| {
                lesson.field_notes.clear();
                lesson
            }));
        app.insert_resource(bare);
    });
    assert!(entity_by_name(&mut app, "Menu Field Note").is_none());
    assert_eq!(visibility(&mut app, "Menu Aside"), Visibility::Visible);
}

/// The note carries the way to switch itself off, beside the way into the
/// lesson. It writes the SETTING, so the answer survives a restart, and the
/// corner keeps the offer above it - two notices, two switches.
#[test]
fn dont_show_again_takes_the_field_note_down() {
    let mut app = training_app();
    assert_eq!(
        visibility(&mut app, "Menu Field Note"),
        Visibility::Inherited
    );

    click(&mut app, "Menu Field Note Dismiss");

    assert_eq!(
        *app.world().resource::<FieldNoteSetting>(),
        FieldNoteSetting::Hidden
    );
    assert_eq!(visibility(&mut app, "Menu Field Note"), Visibility::Hidden);
    assert_eq!(
        visibility(&mut app, "Training Prompt"),
        Visibility::Inherited,
        "switching the notes off is not answering the first-launch offer"
    );
    assert_eq!(visibility(&mut app, "Menu Aside"), Visibility::Visible);
}

/// The player who switched the notes back on in Settings gets them back.
///
/// `Inherited`, not `Visible`, for the same reason the offer is: the corner
/// decides whether anything in it is on screen, and a card that asserted
/// itself Visible would survive a modal.
#[test]
fn the_setting_is_what_decides_whether_the_note_draws() {
    let mut app = training_app_with(|app| {
        app.insert_resource(FieldNoteSetting::Hidden);
    });
    assert_eq!(visibility(&mut app, "Menu Field Note"), Visibility::Hidden);

    *app.world_mut().resource_mut::<FieldNoteSetting>() = FieldNoteSetting::Shown;
    app.update();

    assert_eq!(
        visibility(&mut app, "Menu Field Note"),
        Visibility::Inherited
    );
}

/// The modals are 85 percent of the window, so a card at the screen edge would
/// otherwise still be visible - and clickable - beside a screen the player
/// thinks owns the input.
#[test]
fn an_open_modal_takes_the_card_off_the_screen() {
    let mut app = training_app();
    assert_eq!(visibility(&mut app, "Menu Aside"), Visibility::Visible);

    click(&mut app, "Lessons Button");
    assert_eq!(visibility(&mut app, "Menu Aside"), Visibility::Hidden);

    click(&mut app, "Training Back Button");
    assert_eq!(visibility(&mut app, "Menu Aside"), Visibility::Visible);

    click(&mut app, "Scenarios Button");
    assert_eq!(
        visibility(&mut app, "Menu Aside"),
        Visibility::Hidden,
        "the card hides behind EVERY menu modal, not only the handbook's"
    );
}

/// The corner and both its notices are spawned, marked and reachable - a test
/// that queried only by name would pass on a stray entity someone renamed onto.
#[test]
fn the_corner_carries_its_markers() {
    let mut app = training_app();
    let mut asides = app.world_mut().query::<(Entity, &MenuAside)>();
    assert_eq!(asides.iter(app.world()).count(), 1);
    let mut offers = app.world_mut().query::<(Entity, &TrainingPromptCard)>();
    assert_eq!(offers.iter(app.world()).count(), 1);
    let mut notes = app.world_mut().query::<(Entity, &MenuFieldNoteCard)>();
    assert_eq!(notes.iter(app.world()).count(), 1);
    let mut panels = app.world_mut().query::<(Entity, &TrainingPanel)>();
    assert_eq!(panels.iter(app.world()).count(), 1);
}

/// Selecting a lesson MOVES the highlight; it does not rebuild the list.
///
/// The rows used to be despawned and respawned on every click, because one
/// click writes both the selection and the progress record and the list
/// refreshed on either. Sixty-odd rows flashed to move one highlight one place,
/// and the scroll position went with them. Fails if `training_list_dirty`
/// starts reading those two signals again.
#[test]
fn selecting_a_lesson_moves_the_highlight_without_respawning_the_rows() {
    let mut app = training_app();
    let before: Vec<Entity> = row_entities(&mut app);
    assert!(before.len() > 1, "the fixture draws more than one row");

    click(&mut app, "Lesson Row: flight_momentum");
    app.update();

    assert_eq!(
        row_entities(&mut app),
        before,
        "the rows were respawned for a selection that only moved a highlight"
    );
    assert_eq!(selected(&app).as_deref(), Some("flight_momentum"));
    assert_eq!(
        marked_row(&mut app).as_deref(),
        Some("flight_momentum"),
        "the `Selected` highlight did not follow the selection"
    );
}

/// The count over the two panes follows the record even though the list no
/// longer rebuilds. A row that reads READ under a summary that still says
/// "3 opened" is the screen disagreeing with itself.
#[test]
fn opening_a_new_lesson_moves_the_progress_summary() {
    let mut app = training_app();
    let total = app.world().resource::<TrainingCatalog>().len();

    // `combat_turrets` is untouched in the fixture, so opening it moves the
    // opened count; the three lessons already in the record would not.
    click(&mut app, "Lesson Row: combat_turrets");
    app.update();

    let wanted = format!("1 of {total} completed - 4 opened");
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == &wanted),
        "no progress summary reading `{wanted}`: {texts:?}"
    );
}

/// A row that has just been opened grows its READ badge in place.
#[test]
fn a_freshly_opened_lesson_grows_its_badge_without_a_rebuild() {
    let mut app = training_app();
    let row = entity_by_name(&mut app, "Lesson Row: combat_turrets").expect("the row");
    assert!(
        badge_texts(&mut app, row).is_empty(),
        "an untouched lesson must wear no badge"
    );

    click(&mut app, "Lesson Row: combat_turrets");
    app.update();

    assert_eq!(
        entity_by_name(&mut app, "Lesson Row: combat_turrets"),
        Some(row),
        "the row was respawned rather than restated"
    );
    assert_eq!(badge_texts(&mut app, row), vec!["[READ]".to_string()]);
}
