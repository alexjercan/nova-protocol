//! The Mod Hub: what the Sandbox button opens. A plain list of the local mods,
//! most recent first, with New and Import in its header.
//!
//! Nothing here touches a document. Open and New put a [`ModProject`] in the
//! world and move to the editor, which stands the document up from it.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate, Button},
};
use nova_gameplay::prelude::GameStates;
use nova_ui::{
    prelude::{
        button, close_popups, panel, popup_row, scroll_bar, scroll_column, scroll_row,
        scroll_viewport, spawn_popup, text_field, ButtonSpec, DoubleClick, ListRow, PopupRoot,
        TextFieldFocused, TextFieldSpec, TextFieldValue, UiText,
    },
    theme::{self, density, ActiveUiTheme, UiColor},
    widget::{ButtonCueType, Selected, ThemedFill, ThemedRadius, ThemedText},
};

use crate::{
    bundle::{FileWindowKind, FileWindowRequest},
    project::{
        ids::{is_taken, project_id},
        store::{self, HubEntry, CAN_SAVE},
        ModProject, PendingMods,
    },
    template::ScenarioTemplate,
    ui::{
        files::{id_line, PathPurposeType, NAME_CHARS},
        layer,
        window::{window_layer, DestructiveVerb},
    },
    ExampleStates,
};

/// The header: one row of controls.
const HEAD_H: f32 = 34.0;
/// One mod in the list.
const ROW_H: f32 = 26.0;
/// The Version column.
const VERSION_W: f32 = 90.0;
/// The Last Opened column.
const OPENED_W: f32 = 110.0;
/// The side margin of the list.
const HUB_GUTTER: f32 = 12.0;
/// The New Mod modal.
const MODAL_W: f32 = 500.0;

/// What the hub holds across frames.
#[derive(Resource, Default)]
pub(crate) struct HubForm {
    /// The template the next New Mod starts from.
    pub(crate) template: ScenarioTemplate,
    /// The status line in the header.
    pub(crate) line: String,
    /// Moves when the list of mods may have changed.
    pub(crate) revision: u64,
    /// The row Enter opens.
    pub(crate) selected: Option<String>,
}

/// The list of mods.
#[derive(Component)]
pub(crate) struct HubList;

/// The status line of the hub.
#[derive(Component)]
pub(crate) struct HubLine;

/// The New Mod modal, from its catcher down.
#[derive(Component)]
pub(crate) struct HubNewModal;

/// The name field of New Mod.
#[derive(Component)]
pub(crate) struct HubNameField;

/// The line under the name: what id it makes.
#[derive(Component)]
pub(crate) struct HubIdPreview;

/// The Create button.
#[derive(Component)]
pub(crate) struct HubCreateButton;

/// One mod's row, by id.
#[derive(Component, Clone)]
pub(crate) struct HubRow {
    id: String,
    name: String,
    /// A row that cannot open offers no copy either.
    broken: bool,
}

/// What a row's menu does to which mod.
#[derive(Component, Clone)]
pub(crate) enum HubVerb {
    /// Open the mod.
    Open(String),
    /// Copy it under a free name.
    Duplicate(String, String),
    /// Ask, then delete it.
    Delete(String),
}

/// Words of `size` in `colour`.
fn line(text: impl Into<String>, size: f32, colour: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(text.into()),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
    )
}

/// A column of the list: `width` px, or the free width when `None`.
fn column(width: Option<f32>) -> Node {
    Node {
        width: width.map_or(Val::Auto, px),
        flex_grow: if width.is_none() { 1.0 } else { 0.0 },
        flex_basis: if width.is_none() { px(0) } else { Val::Auto },
        flex_shrink: if width.is_none() { 1.0 } else { 0.0 },
        min_width: px(0),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// Build the hub.
pub(crate) fn spawn_hub(mut commands: Commands, mut form: ResMut<HubForm>) {
    form.revision += 1;
    if !CAN_SAVE {
        form.line = "Saving is not available on the web. Work stays in memory.".to_string();
    }
    commands.spawn((
        Name::new("Hub Camera"),
        Camera2d,
        DespawnOnExit(ExampleStates::Hub),
    ));
    commands
        .spawn((
            Name::new("Mod Hub"),
            DespawnOnExit(ExampleStates::Hub),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColor::Void),
        ))
        .with_children(|hub| {
            spawn_head(hub, &form.line);
            spawn_list(hub);
        });
    commands.spawn((window_layer(), DespawnOnExit(ExampleStates::Hub)));
}

/// The header: Back, the title, the status line, Import and New.
fn spawn_head(hub: &mut RelatedSpawnerCommands<ChildOf>, status: &str) {
    hub.spawn((
        Name::new("Hub Head"),
        Node {
            height: px(HEAD_H),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::axes(px(HUB_GUTTER), px(0)),
            border: UiRect::bottom(px(theme::BORDER_W)),
            ..default()
        },
        // Over the game's status bar, which sits in this corner.
        GlobalZIndex(layer::CHROME_Z),
        panel(),
    ))
    .with_children(|head| {
        head.spawn((
            Name::new("Hub Back Button"),
            button(ButtonSpec::new("Back").dense().ghost()),
            ButtonCueType::Back,
            observe(on_back),
        ));
        head.spawn((
            Name::new("Hub Title"),
            line("MODS", density::TEXT, UiColor::Body),
        ));
        head.spawn((Name::new("Hub Line Slot"), column(None)))
            .with_children(|slot| {
                slot.spawn((
                    Name::new("Hub Line"),
                    HubLine,
                    line(status, density::TEXT_SMALL, UiColor::Accent),
                ));
            });
        let mut import = head.spawn((
            Name::new("Hub Import Button"),
            button(ButtonSpec::new("Import").dense()),
        ));
        if CAN_SAVE {
            import.observe(on_import);
        } else {
            import.insert(InteractionDisabled);
        }
        head.spawn((
            Name::new("Hub New Button"),
            button(ButtonSpec::new("New").dense().primary()),
            observe(on_new),
        ));
    });
}

/// The columns' words and the list under them.
fn spawn_list(hub: &mut RelatedSpawnerCommands<ChildOf>) {
    hub.spawn((
        Name::new("Hub Columns"),
        Node {
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            min_height: px(density::ROW_H),
            padding: UiRect::axes(px(HUB_GUTTER + 6.0), px(0)),
            ..default()
        },
    ))
    .with_children(|heads| {
        for (word, width) in [
            ("NAME", None),
            ("VERSION", Some(VERSION_W)),
            ("LAST OPENED", Some(OPENED_W)),
        ] {
            heads
                .spawn(column(width))
                .with_child(line(word, density::TEXT_SMALL, UiColor::Label));
        }
    });
    hub.spawn((
        Name::new("Hub List Scroll"),
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            padding: UiRect::axes(px(HUB_GUTTER), px(0)),
            ..scroll_row()
        },
    ))
    .with_children(|scroll| {
        scroll.spawn((
            Name::new("Hub List"),
            HubList,
            Node {
                align_items: AlignItems::Stretch,
                ..scroll_column()
            },
            scroll_viewport(),
        ));
        scroll.spawn((Name::new("Hub List Scrollbar"), scroll_bar()));
    });
}

/// Open the New Mod modal: a name, a template, Create.
fn spawn_new_modal(commands: &mut Commands, template: ScenarioTemplate) {
    commands
        .spawn((
            Name::new("Hub New Modal"),
            HubNewModal,
            DespawnOnExit(ExampleStates::Hub),
            GlobalZIndex(layer::WINDOW_Z),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ThemedFill::alpha(UiColor::Void, 0.7),
            observe(on_modal_scrim),
        ))
        .with_children(|scrim| {
            scrim
                .spawn((
                    Name::new("Hub New Panel"),
                    Node {
                        width: px(MODAL_W),
                        max_width: percent(94),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|modal| {
                    modal.spawn((
                        Name::new("Hub New Title"),
                        line("NEW MOD", density::TEXT, UiColor::Accent),
                    ));
                    modal.spawn((
                        Name::new("Hub Name Field"),
                        HubNameField,
                        TextFieldFocused::at_end(""),
                        text_field(
                            TextFieldSpec::new("")
                                .max_chars(NAME_CHARS)
                                .placeholder("Name your mod")
                                .tool(),
                        ),
                    ));
                    modal.spawn((
                        Name::new("Hub Id Preview"),
                        HubIdPreview,
                        Node {
                            min_height: px(density::TEXT_SMALL + 3.0),
                            ..default()
                        },
                        line("", density::TEXT_SMALL, UiColor::Label),
                    ));
                    modal
                        .spawn((
                            Name::new("Hub Templates"),
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Stretch,
                                column_gap: px(density::GAP),
                                ..default()
                            },
                        ))
                        .with_children(|tiles| {
                            for choice in ScenarioTemplate::ALL {
                                spawn_template_tile(tiles, choice, choice == template);
                            }
                        });
                    modal
                        .spawn((
                            Name::new("Hub New Foot"),
                            Node {
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: px(density::GAP),
                                ..default()
                            },
                        ))
                        .with_children(|foot| {
                            foot.spawn((
                                Name::new("Hub Cancel Button"),
                                button(ButtonSpec::new("Cancel").dense().ghost()),
                                ButtonCueType::Back,
                                observe(on_modal_cancel),
                            ));
                            foot.spawn((
                                Name::new("Hub Create Button"),
                                HubCreateButton,
                                InteractionDisabled,
                                button(ButtonSpec::new("Create").dense().primary()),
                                observe(on_create),
                            ));
                        });
                });
        });
}

/// One template: its name and what it starts.
fn spawn_template_tile(
    tiles: &mut RelatedSpawnerCommands<ChildOf>,
    template: ScenarioTemplate,
    picked: bool,
) {
    let mut tile = tiles.spawn((
        Name::new(format!("Hub Template {}", template.label())),
        template,
        ListRow,
        Button,
        Hovered::default(),
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            padding: UiRect::all(px(6)),
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        observe(on_hub_template),
    ));
    if picked {
        tile.insert(Selected);
    }
    tile.with_children(|tile| {
        tile.spawn((
            line(template.label(), density::TEXT, UiColor::Primary),
            Pickable::IGNORE,
        ));
        tile.spawn((
            UiText,
            Text::new(template.hint()),
            TextFont {
                font_size: FontSize::Px(density::TEXT_SMALL),
                ..default()
            },
            TextColor(Color::NONE),
            // Not Label: a picked tile's fill drowns it.
            ThemedText::new(UiColor::Body),
            Pickable::IGNORE,
        ));
    });
}

/// `seconds` since the epoch as a date, in UTC.
fn date_of(seconds: u64) -> String {
    // Civil-from-days (Howard Hinnant's public-domain algorithm): a date is all
    // this row needs, which is no reason to take a calendar crate.
    let days = (seconds / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// A name for a copy of `name` that makes a free id.
fn free_copy_name(name: &str, known: &[String]) -> String {
    let mut copy = format!("{name} Copy");
    let mut count = 2;
    while project_id(&copy).is_ok_and(|id| is_taken(&id, known)) {
        copy = format!("{name} Copy {count}");
        count += 1;
    }
    copy
}

/// The hub's per-frame systems, in order, each one's commands landed before
/// the next one reads: the keys, whose Esc despawns the modal; the list, which
/// a Delete or a Duplicate rebuilds; then the marks, read from the rows that
/// now stand. A mark written beside the rebuild went to a row the rebuild had
/// despawned, and Bevy warned `remove::<Selected>` on a despawned entity.
pub(crate) fn hub_systems(
) -> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::ScheduleSystem> {
    (hub_keys, sync_hub_rows, sync_hub_form).chain()
}

/// Rebuild the list when the mods on disk may have changed.
pub(crate) fn sync_hub_rows(
    mut commands: Commands,
    form: Res<HubForm>,
    lists: Query<Entity, With<HubList>>,
    fresh: Query<(), Added<HubList>>,
    mut shown: Local<u64>,
) {
    if fresh.is_empty() && *shown == form.revision {
        return;
    }
    let Ok(list) = lists.single() else {
        return;
    };
    *shown = form.revision;
    let entries = store::hub_entries();
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        if entries.is_empty() {
            list.spawn((
                Name::new("Hub Empty"),
                Node {
                    padding: UiRect::all(px(6)),
                    ..default()
                },
                line(
                    "No mods here yet. Press New to start one.",
                    density::TEXT,
                    UiColor::Label,
                ),
            ));
        }
        for entry in &entries {
            spawn_row(list, entry);
        }
    });
}

/// One mod: name, version, last opened. A broken mod says why in place of its
/// id, because a row that hid it would leave a directory nobody could delete.
fn spawn_row(list: &mut RelatedSpawnerCommands<ChildOf>, entry: &HubEntry) {
    list.spawn((
        Name::new(format!("Hub Row {}", entry.id)),
        HubRow {
            id: entry.id.clone(),
            name: entry.name.clone(),
            broken: entry.fault.is_some(),
        },
        ListRow,
        Button,
        Hovered::default(),
        Node {
            min_height: px(ROW_H),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::axes(px(6), px(0)),
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        observe(on_hub_row),
        observe(on_hub_row_menu),
    ))
    .with_children(|row| {
        row.spawn(Node {
            column_gap: px(10),
            align_items: AlignItems::Baseline,
            ..column(None)
        })
        .with_children(|name| {
            name.spawn((
                line(entry.name.clone(), density::TEXT + 1.0, UiColor::Primary),
                Pickable::IGNORE,
            ));
            let (detail, tint) = entry.fault.as_ref().map_or_else(
                || (entry.id.clone(), UiColor::Label),
                |fault| (fault.clone(), UiColor::Danger),
            );
            name.spawn((line(detail, density::TEXT_SMALL, tint), Pickable::IGNORE));
        });
        row.spawn((column(Some(VERSION_W)), Pickable::IGNORE))
            .with_child((
                line(
                    format!("v{}", entry.version),
                    density::TEXT,
                    UiColor::Secondary,
                ),
                Pickable::IGNORE,
            ));
        let opened = entry.opened.map_or_else(|| "-".to_string(), date_of);
        row.spawn((column(Some(OPENED_W)), Pickable::IGNORE))
            .with_child((
                line(opened, density::TEXT, UiColor::Secondary),
                Pickable::IGNORE,
            ));
    });
}

/// Keep the id preview, the status line, the marks and Create true.
#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system; each query is one widget of the form"
)]
pub(crate) fn sync_hub_form(
    mut commands: Commands,
    theme: Res<ActiveUiTheme>,
    form: Res<HubForm>,
    names: Query<&TextFieldValue, With<HubNameField>>,
    mut previews: Query<(&mut Text, &mut TextColor), (With<HubIdPreview>, Without<HubLine>)>,
    mut lines: Query<&mut Text, With<HubLine>>,
    creates: Query<(Entity, Has<InteractionDisabled>), With<HubCreateButton>>,
    choices: Query<(Entity, &ScenarioTemplate, Has<Selected>)>,
    rows: Query<(Entity, &HubRow, Has<Selected>)>,
    mut known: Local<(u64, Vec<String>)>,
) {
    if known.0 != form.revision {
        *known = (form.revision, store::local_ids());
    }
    let typed = names.iter().next().map_or("", |value| value.0.as_str());
    let (preview, colour, usable) = if typed.trim().is_empty() {
        (String::new(), UiColor::Label, false)
    } else {
        id_line(typed, &known.1)
    };
    let colour = theme.color(colour);
    for (mut text, mut paint) in &mut previews {
        if text.0 != preview {
            text.0.clone_from(&preview);
        }
        if paint.0 != colour {
            paint.0 = colour;
        }
    }
    for mut text in &mut lines {
        if text.0 != form.line {
            text.0.clone_from(&form.line);
        }
    }
    for (button, choice, marked) in &choices {
        match (*choice == form.template, marked) {
            (true, false) => {
                commands.entity(button).insert(Selected);
            }
            (false, true) => {
                commands.entity(button).remove::<Selected>();
            }
            _ => {}
        }
    }
    for (row, entry, marked) in &rows {
        match (form.selected.as_deref() == Some(entry.id.as_str()), marked) {
            (true, false) => {
                commands.entity(row).insert(Selected);
            }
            (false, true) => {
                commands.entity(row).remove::<Selected>();
            }
            _ => {}
        }
    }
    for (button, greyed) in &creates {
        match (usable, greyed) {
            (true, true) => {
                commands.entity(button).remove::<InteractionDisabled>();
            }
            (false, false) => {
                commands.entity(button).insert(InteractionDisabled);
            }
            _ => {}
        }
    }
}

/// The hub's keys. Enter creates from the modal, or opens the marked row; Esc
/// shuts the modal or a row menu. In the modal Up and Down step the template:
/// Left and Right are the name field's caret.
#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system; each query is one surface the keys answer to"
)]
pub(crate) fn hub_keys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    modals: Query<Entity, With<HubNewModal>>,
    creates: Query<Has<InteractionDisabled>, With<HubCreateButton>>,
    names: Query<&TextFieldValue, With<HubNameField>>,
    popups: Query<Entity, With<PopupRoot>>,
    windows: Query<(), With<crate::ui::window::EditorWindow>>,
    mut form: ResMut<HubForm>,
) {
    if !popups.is_empty() {
        if keys.just_pressed(KeyCode::Escape) {
            close_popups(&mut commands, &popups);
        }
        return;
    }
    if !windows.is_empty() {
        return;
    }
    if let Ok(modal) = modals.single() {
        let step = |template| {
            let at = ScenarioTemplate::ALL
                .iter()
                .position(|choice| *choice == template)
                .unwrap_or_default();
            if keys.just_pressed(KeyCode::ArrowUp) {
                ScenarioTemplate::ALL[at.saturating_sub(1)]
            } else if keys.just_pressed(KeyCode::ArrowDown) {
                ScenarioTemplate::ALL[(at + 1).min(ScenarioTemplate::ALL.len() - 1)]
            } else {
                template
            }
        };
        let template = step(form.template);
        if template != form.template {
            form.template = template;
        }
        if keys.just_pressed(KeyCode::Escape) {
            commands.entity(modal).despawn();
        } else if keys.just_pressed(KeyCode::Enter)
            && creates.iter().next().is_some_and(|greyed| !greyed)
        {
            queue_create(&mut commands, &names, form.template);
        }
    } else if keys.just_pressed(KeyCode::Enter) {
        if let Some(id) = form.selected.clone() {
            commands.queue(move |world: &mut World| open_mod(world, &id));
        }
    }
}

/// New: the modal, unless it stands.
fn on_new(
    _activate: On<Activate>,
    mut commands: Commands,
    form: Res<HubForm>,
    modals: Query<(), With<HubNewModal>>,
) {
    if modals.is_empty() {
        spawn_new_modal(&mut commands, form.template);
    }
}

/// A press on the dim around the modal puts it away.
fn on_modal_scrim(
    press: On<PointerPress>,
    mut commands: Commands,
    modals: Query<Entity, With<HubNewModal>>,
) {
    if let Ok(modal) = modals.get(press.original_event_target()) {
        commands.entity(modal).despawn();
    }
}

/// Cancel.
fn on_modal_cancel(
    _activate: On<Activate>,
    mut commands: Commands,
    modals: Query<Entity, With<HubNewModal>>,
) {
    for modal in &modals {
        commands.entity(modal).despawn();
    }
}

/// A template tile.
fn on_hub_template(
    activate: On<Activate>,
    templates: Query<&ScenarioTemplate>,
    mut form: ResMut<HubForm>,
) {
    if let Ok(template) = templates.get(activate.entity) {
        form.template = *template;
    }
}

/// Create the typed mod.
fn queue_create(
    commands: &mut Commands,
    names: &Query<&TextFieldValue, With<HubNameField>>,
    template: ScenarioTemplate,
) {
    let name = names
        .iter()
        .next()
        .map_or_else(String::new, |value| value.0.trim().to_string());
    commands.queue(move |world: &mut World| create_mod(world, &name, template));
}

/// Create: open a new mod. The editor founds its first scenario and writes it,
/// so what is listed in the hub always holds one.
fn on_create(
    _activate: On<Activate>,
    mut commands: Commands,
    names: Query<&TextFieldValue, With<HubNameField>>,
    form: Res<HubForm>,
) {
    queue_create(&mut commands, &names, form.template);
}

/// Start the mod `name` and go to the editor.
///
/// A name is all a mod needs: the rest of its manifest is Mod Settings' to fill.
pub(crate) fn create_mod(world: &mut World, name: &str, template: ScenarioTemplate) {
    let id = if CAN_SAVE {
        match store::copy_id(name) {
            Ok(id) => Some(id),
            Err(line) => {
                world.resource_mut::<HubForm>().line = line;
                return;
            }
        }
    } else {
        None
    };
    world.insert_resource(ModProject::new(id, name, "", template));
    world
        .resource_mut::<NextState<ExampleStates>>()
        .set(ExampleStates::Editor);
}

/// Open the mod `id` and go to the editor, or say why not.
pub(crate) fn open_mod(world: &mut World, id: &str) {
    match store::read_project(id) {
        Ok(project) => {
            store::remember_project(id);
            world.insert_resource(project);
            world
                .resource_mut::<NextState<ExampleStates>>()
                .set(ExampleStates::Editor);
        }
        Err(fault) => {
            let mut form = world.resource_mut::<HubForm>();
            form.line = format!("{id} did not open: {fault}");
            form.revision += 1;
        }
    }
}

/// Delete the mod `id`, and take it out of the enabled set on the way out.
pub(crate) fn delete_local_mod(world: &mut World, id: &str) {
    let done = store::delete_mod(id);
    let deleted = done.is_ok();
    let line = match done {
        Ok(()) => {
            world.get_resource_or_init::<PendingMods>().disable(id);
            format!("deleted {id}")
        }
        Err(fault) => format!("{id} was not deleted: {fault}"),
    };
    let mut form = world.resource_mut::<HubForm>();
    if deleted && form.selected.as_deref() == Some(id) {
        form.selected = None;
    }
    form.line = line;
    form.revision += 1;
}

/// Close the open mod and go back to the hub. Asked before, when there is work
/// to lose. The document goes on entering the hub.
pub(crate) fn leave_to_hub(world: &mut World) {
    world
        .resource_mut::<NextState<ExampleStates>>()
        .set(ExampleStates::Hub);
}

/// A press on a row marks it. The second of a double opens it.
fn on_hub_row(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&HubRow>,
    time: Res<Time<Real>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut last: ResMut<DoubleClick<String>>,
    mut form: ResMut<HubForm>,
) {
    // `Button` activates on any button. The right one is the context menu's.
    if mouse.just_released(MouseButton::Right) {
        return;
    }
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    form.selected = Some(row.id.clone());
    if last.press(row.id.clone(), time.elapsed_secs()) {
        let id = row.id.clone();
        commands.queue(move |world: &mut World| open_mod(world, &id));
    }
}

/// Right click on a row: Open, Duplicate and Delete at the pointer.
fn on_hub_row_menu(
    click: On<PointerClick>,
    mut commands: Commands,
    rows: Query<&HubRow>,
    roots: Query<Entity, With<PopupRoot>>,
    mut form: ResMut<HubForm>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let Ok(row) = rows.get(click.event_target()) else {
        return;
    };
    form.selected = Some(row.id.clone());
    let copy = free_copy_name(&row.name, &store::local_ids());
    let row = row.clone();
    close_popups(&mut commands, &roots);
    let popup = spawn_popup(
        &mut commands,
        click.pointer.position,
        layer::MENU_Z,
        |menu| {
            if !row.broken {
                menu.spawn((
                    Name::new(format!("Hub Menu Open {}", row.id)),
                    HubVerb::Open(row.id.clone()),
                    popup_row("Open", "Enter"),
                    observe(on_hub_verb),
                ));
                menu.spawn((
                    Name::new(format!("Hub Menu Duplicate {}", row.id)),
                    HubVerb::Duplicate(row.id.clone(), copy),
                    popup_row("Duplicate", ""),
                    observe(on_hub_verb),
                ));
            }
            menu.spawn((
                Name::new(format!("Hub Menu Delete {}", row.id)),
                HubVerb::Delete(row.id.clone()),
                popup_row("Delete", ""),
                observe(on_hub_verb),
            ));
        },
    );
    commands
        .entity(popup)
        .insert(DespawnOnExit(ExampleStates::Hub));
}

/// A row menu's Open, Duplicate or Delete.
fn on_hub_verb(
    activate: On<Activate>,
    mut commands: Commands,
    verbs: Query<&HubVerb>,
    mut confirm: crate::ui::window::ConfirmAsk,
) {
    let Ok(verb) = verbs.get(activate.entity) else {
        return;
    };
    match verb.clone() {
        HubVerb::Open(id) => commands.queue(move |world: &mut World| open_mod(world, &id)),
        HubVerb::Duplicate(id, name) => commands.queue(move |world: &mut World| {
            let line = match store::duplicate_mod(&id, &name) {
                Ok(copy) => format!("copied as {copy}"),
                Err(fault) => format!("{id} was not copied: {fault}"),
            };
            let mut form = world.resource_mut::<HubForm>();
            form.line = line;
            form.revision += 1;
        }),
        HubVerb::Delete(id) => confirm.ask(DestructiveVerb::DeleteMod(id)),
    }
}

/// Import Package.
fn on_import(_activate: On<Activate>, mut window: ResMut<FileWindowRequest>) {
    window.0 = Some(FileWindowKind::PickPath(PathPurposeType::Package));
}

/// Back: the main menu.
fn on_back(_activate: On<Activate>, mut state: ResMut<NextState<GameStates>>) {
    state.set(GameStates::MainMenu);
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use bevy::{
        log::tracing_subscriber::{self, util::SubscriberInitExt},
        prelude::*,
        state::app::StatesPlugin,
    };
    use nova_ui::{theme::ActiveUiTheme, widget::Selected};

    use super::{create_mod, hub_systems, HubForm, HubList, HubRow};
    use crate::{project::ModProject, template::ScenarioTemplate, ExampleStates};

    /// A buffer this thread's log writes to.
    #[derive(Clone, Default)]
    struct Said(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Said {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("unpoisoned").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Deleting the marked mod moves the list's revision and clears the mark
    /// in one frame. The marks are read after the rebuild lands, so none is
    /// taken off a row the rebuild despawned. `remove` bakes the WARN handler
    /// in when it is queued, so the log is the only place the fault shows.
    #[test]
    fn deleting_the_marked_mod_takes_no_mark_off_a_despawned_row() {
        let said = Said::default();
        let writer = said.clone();
        let _guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ExampleStates>()
            .init_resource::<HubForm>()
            .init_resource::<ActiveUiTheme>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, hub_systems());
        let list = app.world_mut().spawn(HubList).id();
        app.update();
        let row = app
            .world_mut()
            .spawn((
                HubRow {
                    id: "gone".to_string(),
                    name: "Gone".to_string(),
                    broken: false,
                },
                Selected,
                ChildOf(list),
            ))
            .id();
        app.world_mut().resource_mut::<HubForm>().selected = Some("gone".to_string());
        app.update();

        let mut form = app.world_mut().resource_mut::<HubForm>();
        form.selected = None;
        form.revision += 1;
        app.update();

        assert!(app.world().get_entity(row).is_err(), "the list was rebuilt");
        let said = String::from_utf8_lossy(&said.0.lock().expect("unpoisoned")).into_owned();
        assert!(!said.contains("Selected"), "{said}");
    }

    #[test]
    fn a_new_mod_needs_only_a_name() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .init_state::<ExampleStates>()
            .init_resource::<HubForm>();

        create_mod(app.world_mut(), "Red Run", ScenarioTemplate::default());

        let project = app.world().resource::<ModProject>();
        assert_eq!(project.data.meta.name, "Red Run");
        assert_eq!(
            project.data.meta.author, "",
            "author waits for Mod Settings"
        );
    }
}
